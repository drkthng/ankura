<script lang="ts">
  // The daily session (ADR 0019): today's words (backlog rescue), today's text (an episode of a
  // serial story or a single text), read and listen, questions, cards.
  import { onMount } from "svelte";
  import { api, type Pick, type SeriesInfo, type TodayOverview } from "../lib/api";
  import Quiz from "../components/Quiz.svelte";

  let { onopen, ongo }: { onopen: (id: string) => void; ongo: (view: "cards" | "guide") => void } = $props();

  let overview = $state<TodayOverview | null>(null);
  let streak = $state(0);
  let error = $state("");
  let adding = $state("");
  let writing = $state(false);
  let showQuiz = $state(false);
  let textTitle = $state("");

  // Today's text: continue a series, start one, or a single text.
  let mode = $state<"continue" | "new" | "single">("new");
  let seriesId = $state("");
  let newTitle = $state("");
  let topic = $state("");
  let length = $state(500);
  let polite = $state(false);

  const REASON: Record<string, string> = {
    fading: "fading in Anki",
    stealth_leech: "keeps slipping",
    quiz_missed: "missed in questions",
    looked_up: "looked up",
    learning: "learning in Anki",
    added: "your pick",
  };

  async function load() {
    try {
      overview = await api.todayOverview();
      streak = (await api.readingStats().catch(() => null))?.streak ?? 0;
      if (overview.series.length) {
        mode = "continue";
        seriesId ||= overview.series[0].series.id;
      }
      const id = overview.session.text_id;
      textTitle = id ? ((await api.listTexts().catch(() => [])).find((t) => t.id === id)?.title ?? "") : "";
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  const session = $derived(overview?.session);
  const done = (s: string) => !!session?.done.includes(s);
  const chosenSeries = $derived<SeriesInfo | undefined>(overview?.series.find((x) => x.series.id === seriesId));
  const finished = $derived(done("read") && done("questions"));

  async function saveSession() {
    if (session) await api.todaySave(session);
  }
  function remove(p: Pick) {
    if (!session) return;
    session.words = session.words.filter((w) => w !== p);
    saveSession();
  }
  function add() {
    const w = adding.trim();
    if (!session || !w) return;
    session.words = [...session.words, { key: { lang: "ja", lemma: w, reading: "" }, display: w, reason: "added" }];
    adding = "";
    saveSession();
  }
  async function otherWords() {
    if (!overview) return;
    overview.session = await api.todayNewWords();
  }

  async function write() {
    if (!session) return;
    error = "";
    writing = true;
    try {
      let series: string | null = null;
      let len = length;
      let pol = polite;
      let top = topic.trim();
      if (mode === "continue" && chosenSeries) {
        series = chosenSeries.series.id;
        len = chosenSeries.series.length;
        pol = chosenSeries.series.polite;
        top = "";
      } else if (mode === "new") {
        const s = await api.seriesCreate(newTitle.trim() || top || "わたしの物語", top, pol, len);
        series = s.id;
      }
      const r = await api.generateText({ targets: session.words.map((w) => w.display), topic: top, length: len, polite: pol, series });
      session.text_id = r.text.id;
      textTitle = r.text.title;
      await saveSession();
      onopen(r.text.id);
    } catch (e) {
      error = String(e);
    } finally {
      writing = false;
    }
  }

  async function mark(step: string) {
    if (!overview) return;
    overview.session = await api.todayMark(step);
  }
</script>

<div class="page">
  <h1>Today</h1>
  {#if error}<p class="error">{error}</p>{/if}
  {#if overview && session}
    <p class="muted lead">
      {#if streak > 0}Reading streak: <strong>{streak} day{streak === 1 ? "" : "s"}</strong>. {/if}
      {#if overview.rescue.fading > 0}
        {overview.rescue.fading.toLocaleString()} words are fading in Anki; this week you met
        <strong>{overview.rescue.met_this_week}</strong> of them in texts.
      {/if}
      <button class="link" onclick={() => ongo("guide")}>How to learn</button>
    </p>

    {#if finished}
      <section class="card done-card">
        <p><strong>Done for today.</strong> Reading again later, or listening once more, still counts.</p>
      </section>
    {/if}

    <section class="card step" class:complete={!!session.text_id}>
      <h2><span class="n">1</span> Your words</h2>
      <p class="muted small">
        Fading Anki words first (reading them now makes your overdue reviews easier), then words that keep slipping or
        that you missed.
      </p>
      <div class="chips">
        {#each session.words as w}
          <span class="chip" title={REASON[w.reason] ?? w.reason}>
            <span class="w">{w.display}</span><span class="why">{REASON[w.reason] ?? w.reason}</span>
            {#if !session.text_id}<button class="x" aria-label="Remove {w.display}" onclick={() => remove(w)}>×</button>{/if}
          </span>
        {:else}
          <span class="muted">No suggestions yet: add a few words you want to practise.</span>
        {/each}
      </div>
      {#if !session.text_id}
        <div class="row">
          <input placeholder="Add a word" bind:value={adding} onkeydown={(e) => e.key === "Enter" && add()} />
          <button onclick={add} disabled={!adding.trim()}>Add</button>
          <button class="ghost" onclick={otherWords}>Other suggestions</button>
        </div>
      {/if}
    </section>

    <section class="card step" class:complete={!!session.text_id}>
      <h2><span class="n">2</span> Today's text</h2>
      {#if session.text_id}
        <p>
          <strong class="ja">{textTitle || "Today's text"}</strong>
          <button class="primary" onclick={() => onopen(session.text_id!)}>Open</button>
        </p>
      {:else}
        <div class="row modes">
          {#if overview.series.length}
            <label><input type="radio" bind:group={mode} value="continue" /> Continue a story</label>
          {/if}
          <label><input type="radio" bind:group={mode} value="new" /> Start a new story</label>
          <label><input type="radio" bind:group={mode} value="single" /> A single text</label>
        </div>
        {#if mode === "continue"}
          <div class="row">
            <select bind:value={seriesId}>
              {#each overview.series as s}
                <option value={s.series.id}>{s.series.title} · episode {s.episodes.length + 1}</option>
              {/each}
            </select>
          </div>
          {#if chosenSeries?.episodes.length}
            <p class="muted small">
              Last time: {chosenSeries.episodes[chosenSeries.episodes.length - 1].summary || chosenSeries.episodes[chosenSeries.episodes.length - 1].title}.
              Words from earlier episodes come back.
            </p>
          {/if}
        {:else}
          <div class="row">
            {#if mode === "new"}<input placeholder="Story title (optional)" bind:value={newTitle} />{/if}
            <input class="grow" placeholder="Topic (optional; else from your interests)" bind:value={topic} />
          </div>
          <div class="row">
            <select bind:value={length}>
              <option value={300}>short (~300 characters)</option>
              <option value={500}>medium (~500)</option>
              <option value={900}>long (~900)</option>
            </select>
            <label><input type="checkbox" bind:checked={polite} /> です・ます</label>
          </div>
        {/if}
        <button class="primary big" onclick={write} disabled={writing || !session.words.length}>
          {writing ? "Writing today's text… (about a minute)" : "Write today's text"}
        </button>
      {/if}
    </section>

    <section class="card step" class:complete={done("read") && done("listened")}>
      <h2><span class="n">3</span> Read, then listen</h2>
      <p class="muted small">
        Read at your own pace and look words up. Then read it again while it's read aloud (<kbd>P</kbd>), at the voice's
        pace, without stopping.
      </p>
      <p>
        <span class:ok={done("read")}>{done("read") ? "✓" : "○"} read</span> ·
        <span class:ok={done("listened")}>{done("listened") ? "✓" : "○"} listened</span>
        {#if session.text_id}<button onclick={() => onopen(session.text_id!)}>Open the text</button>{/if}
      </p>
    </section>

    <section class="card step" class:complete={done("questions")}>
      <h2><span class="n">4</span> Questions</h2>
      {#if showQuiz && session.text_id}
        <Quiz textId={session.text_id} ondone={() => mark("questions")} />
      {:else}
        <p class="muted small">Three or four quick questions. Missed words come back tomorrow.</p>
        <button disabled={!session.text_id} onclick={() => (showQuiz = true)}>{done("questions") ? "Again" : "Start"}</button>
      {/if}
    </section>

    <section class="card step" class:complete={done("cards")}>
      <h2><span class="n">5</span> Cards</h2>
      <p class="muted small">Check the cards you made while reading, change them if needed, and add them to Anki.</p>
      <button
        onclick={() => {
          mark("cards");
          ongo("cards");
        }}>Open Cards</button
      >
    </section>
  {:else if !error}
    <p class="muted">Getting today ready…</p>
  {/if}
</div>

<style>
  .page {
    max-width: 820px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .lead {
    margin: -0.4rem 0 0;
  }
  .small {
    font-size: 0.85rem;
  }
  .ja {
    font-family: var(--font-read);
  }
  h2 {
    font-size: 1.05rem;
    margin: 0 0 0.4rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .n {
    display: inline-grid;
    place-items: center;
    width: 1.6rem;
    height: 1.6rem;
    border-radius: 50%;
    background: var(--surface-2);
    font-size: 0.85rem;
  }
  .step.complete .n {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .step .row + .row {
    margin-top: 0.5rem;
  }
  .modes {
    margin-bottom: 0.5rem;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 0.6rem;
  }
  .chip {
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
    border: 1px solid var(--line);
    border-radius: 999px;
    padding: 0.2rem 0.5rem 0.2rem 0.8rem;
    background: var(--surface);
  }
  .chip .w {
    font-family: var(--font-read);
    font-size: 1.1rem;
  }
  .chip .why {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .x {
    border: none;
    background: none;
    padding: 0 0.2rem;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  .big {
    margin-top: 0.8rem;
    font-size: 1.02rem;
    padding: 0.55em 1.4em;
  }
  .ok {
    color: var(--accent);
    font-weight: 600;
  }
  .done-card {
    border-color: var(--accent);
  }
  .link {
    border: none;
    background: none;
    color: var(--accent);
    padding: 0;
    text-decoration: underline;
  }
  kbd {
    font-family: var(--font-ui);
    font-size: 0.75rem;
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 0.35em;
  }
</style>
