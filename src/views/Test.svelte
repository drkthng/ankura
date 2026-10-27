<script lang="ts">
  import { onMount } from "svelte";
  import { abilityName, api, type Estimate, type TestIntro, type TestItem } from "../lib/api";

  type Phase = "intro" | "vocab" | "kanji" | "grammar1" | "grammar2" | "reading" | "loading" | "result";
  let phase = $state<Phase>("intro");
  let intro = $state<TestIntro | null>(null);
  let level = $state(5);
  let error = $state("");
  let loadingWhat = $state("");

  let testId = "";
  let items = $state<TestItem[]>([]);
  let at = $state(0);
  let answers: { item_id: string; choice: number }[] = [];
  let result = $state<Estimate | null>(null);
  let grammar1: Promise<TestItem[]> | null = null;

  onMount(async () => {
    try {
      intro = await api.testIntro();
      level = intro.suggested;
    } catch (e) {
      error = String(e);
    }
  });

  const SECTIONS: Record<string, string> = {
    vocab: "1. Words",
    kanji: "2. Kanji readings",
    grammar1: "3. Grammar",
    grammar2: "3. Grammar, part 2",
    reading: "4. Reading",
  };

  async function start() {
    error = "";
    try {
      const t = await api.testStart(level);
      testId = t.id;
      // The AI writes the first grammar part while the learner does the words.
      grammar1 = api.testGrammar(testId, 1);
      grammar1.catch(() => {});
      begin("vocab", t.vocab);
      kanjiItems = t.kanji;
    } catch (e) {
      error = String(e);
    }
  }
  let kanjiItems: TestItem[] = [];

  function begin(p: Phase, list: TestItem[]) {
    phase = p;
    items = list;
    at = 0;
    answers = [];
  }

  async function wait(what: string, f: () => Promise<TestItem[]>, next: Phase) {
    phase = "loading";
    loadingWhat = what;
    try {
      const list = await f();
      if (list.length) begin(next, list);
      else await advance(next);
    } catch (e) {
      error = String(e);
      phase = "intro";
    }
  }

  async function choose(choice: number) {
    const it = items[at];
    if (!it) return;
    answers.push({ item_id: it.id, choice });
    if (at + 1 < items.length) {
      at += 1;
      return;
    }
    const done = phase;
    await api.testAnswer(testId, answers);
    await advance(done);
  }

  async function advance(done: Phase) {
    if (done === "vocab") begin("kanji", kanjiItems);
    else if (done === "kanji") await wait("Writing grammar questions at your starting level…", () => grammar1!, "grammar1");
    else if (done === "grammar1") await wait("Writing the next grammar questions…", () => api.testGrammar(testId, 2), "grammar2");
    else if (done === "grammar2") await wait("Writing reading passages at your level…", () => api.testReading(testId), "reading");
    else await finish();
  }

  async function finish() {
    phase = "loading";
    loadingWhat = "Working out your level…";
    try {
      result = await api.testFinish(testId);
      phase = "result";
      intro = await api.testIntro();
    } catch (e) {
      error = String(e);
      phase = "intro";
    }
  }

  function onkey(e: KeyboardEvent) {
    if (!["vocab", "kanji", "grammar1", "grammar2", "reading"].includes(phase)) return;
    if (phase === "vocab") {
      if (e.key === "k" || e.key === "ArrowRight") choose(0);
      if (e.key === "d" || e.key === "ArrowLeft") choose(1);
    } else if (/^[1-4]$/.test(e.key) && items[at] && Number(e.key) <= items[at].options.length) {
      choose(Number(e.key) - 1);
    }
  }

  const pct = (x: number) => `${Math.round(x * 100)}%`;
  const best = $derived(result ? result.probs[result.level] : 0);
  const item = $derived(items[at]);
  const levels = [5, 4, 3, 2, 1];
  /** Share right; for words, corrected for saying yes to made-up words. */
  const share = (r: Estimate, section: string, right: number, total: number) => {
    const h = right / Math.max(1, total);
    if (section !== "vocab") return h;
    return r.false_alarms < 1 ? Math.max(0, (h - r.false_alarms) / (1 - r.false_alarms)) : 0;
  };
  const cell = (r: Estimate, section: string, lvl: number) => r.sections.find((s) => s.section === section && s.level === lvl);
</script>

<svelte:window onkeydown={onkey} />

<div class="page">
  <h1>Level check</h1>
  {#if error}<p class="error">{error}</p>{/if}

  {#if phase === "intro"}
    <p class="muted lead">
      About 15-20 minutes. Four parts in the style of the JLPT: words, kanji readings, grammar and reading. The
      grammar adapts to how the first part goes. The result is your likely JLPT level, how sure Ankura is, and what it
      means in CEFR terms. Questions are Ankura's own; grammar and reading are written by the AI and checked by it.
    </p>
    {#if intro}
      <section class="card">
        <h2>Where to start</h2>
        <div class="cov">
          {#each intro.coverage as c}
            <div class="cov-row">
              <span class="lvl">N{c.level}</span>
              <div class="bar"><div style:width="{(100 * c.known_in_anki) / Math.max(1, c.words)}%"></div></div>
              <span class="muted small">{c.known_in_anki.toLocaleString()} of {c.words.toLocaleString()} words in your Anki</span>
            </div>
          {/each}
        </div>
        <div class="row">
          <span>Start at</span>
          {#each levels as l}
            <button class="lvlbtn" class:on={level === l} onclick={() => (level = l)}>N{l}</button>
          {/each}
          {#if level === intro.suggested}<span class="muted small">suggested from your Anki</span>{/if}
        </div>
        <p class="muted small">Words your Anki already knows aren't asked again; they count as known.</p>
        <button class="primary big" onclick={start}>Start</button>
      </section>

      {#if intro.history.length}
        <section class="card">
          <h2>Earlier results</h2>
          <ul class="hist">
            {#each intro.history as h}
              <li>
                <strong>{abilityName(h.result.level)}</strong>
                <span class="muted">({pct(h.result.probs[h.result.level])} likely · CEFR {h.result.cefr} · {new Date(h.finished_at).toLocaleDateString()})</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      <p class="muted small">{intro.credit}</p>
    {/if}
  {:else if phase === "loading"}
    <section class="card center">
      <div class="spinner" aria-hidden="true"></div>
      <p>{loadingWhat}</p>
      <p class="muted small">This takes about half a minute.</p>
    </section>
  {:else if phase === "result" && result}
    <section class="card">
      <p class="muted small">Your level</p>
      <p class="headline"><strong>{abilityName(result.level)}</strong> <span class="muted">· CEFR {result.cefr}</span></p>
      <p class="muted">{pct(best)} likely{best < 0.6 ? "; the levels next to it are possible too" : ""}.</p>
      <div class="chart" role="img" aria-label="How likely each level is">
        {#each result.probs as p, a}
          <div class="col" title="{abilityName(a)}: {pct(p)}">
            <span class="val">{p >= 0.05 ? pct(p) : ""}</span>
            <div class="pbar" class:top={a === result.level} style:height="{Math.max(2, p * 120)}px"></div>
            <span class="lab">{abilityName(a)}</span>
          </div>
        {/each}
      </div>
    </section>

    <section class="card">
      <h2>By part</h2>
      <table class="by">
        <thead><tr><th></th>{#each levels as l}<th>N{l}</th>{/each}</tr></thead>
        <tbody>
          {#each [["vocab", "Words"], ["kanji", "Kanji readings"], ["grammar", "Grammar"], ["reading", "Reading"]] as [key, label]}
            <tr>
              <td>{label}</td>
              {#each levels as l}
                {@const c = cell(result, key, l)}
                {@const r = c ? share(result, key, c.right, c.total) : 0}
                <td class:good={c && r >= 0.7} class:weak={c && r < 0.5}>{c ? `${c.right}/${c.total}` : ""}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
      <p>
        You know about <strong>{result.vocab_known.toLocaleString()}</strong> of the {result.vocab_total.toLocaleString()} words on the JLPT lists
        <span class="muted">({result.vocab_range[0].toLocaleString()}-{result.vocab_range[1].toLocaleString()})</span>.
        {#if result.false_alarms > 0.2}<span class="muted">You said yes to {pct(result.false_alarms)} of the made-up words, which lowers this estimate.</span>{/if}
      </p>
      <p class="muted small">
        Provisional: the questions haven't been tried on other learners yet, so the probabilities rest on assumptions. The
        CEFR level is the Japan Foundation's reference for each JLPT level (reading and listening only). Take the check again
        in a month or so; the questions will be different.
      </p>
      <button onclick={() => (phase = "intro")}>Done</button>
    </section>
  {:else if item}
    <div class="progress"><div style:width="{(100 * at) / items.length}%"></div></div>
    <p class="muted small">{SECTIONS[phase]} · {at + 1} of {items.length}</p>
    {#if phase === "vocab"}
      <section class="card center q">
        <p class="muted small">Do you know this word? Some are made up, so only say yes if you do.</p>
        <p class="word">{item.prompt}</p>
        <div class="row center-row">
          <button class="big" onclick={() => choose(1)}>Don't know <kbd>D</kbd></button>
          <button class="primary big" onclick={() => choose(0)}>Know it <kbd>K</kbd></button>
        </div>
      </section>
    {:else}
      <section class="card q">
        {#if item.passage}<div class="passage">{item.passage}</div>{/if}
        {#if phase === "kanji"}
          <p class="muted small">How is this read?</p>
          <p class="word">{item.prompt}</p>
        {:else}
          <p class="sentence">{item.prompt}</p>
        {/if}
        <div class="options">
          {#each item.options as o, i}
            <button onclick={() => choose(i)}><kbd>{i + 1}</kbd> {o}</button>
          {/each}
        </div>
      </section>
    {/if}
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
  .cov {
    display: grid;
    gap: 0.35rem;
    margin-bottom: 0.8rem;
  }
  .cov-row {
    display: grid;
    grid-template-columns: 2.5rem 1fr 16rem;
    gap: 0.6rem;
    align-items: center;
  }
  .bar,
  .progress {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar div,
  .progress div {
    height: 100%;
    background: var(--accent);
  }
  .lvlbtn.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .big {
    font-size: 1.05rem;
    padding: 0.6em 1.6em;
    margin-top: 0.6rem;
  }
  .center {
    text-align: center;
  }
  .center-row {
    justify-content: center;
  }
  .q {
    padding: 1.6rem;
  }
  .word {
    font-family: var(--font-read);
    font-size: 2.6rem;
    margin: 0.6rem 0 1rem;
    text-align: center;
  }
  .sentence {
    font-family: var(--font-read);
    font-size: 1.4rem;
    line-height: 1.8;
  }
  .passage {
    font-family: var(--font-read);
    font-size: 1.1rem;
    line-height: 1.9;
    white-space: pre-line;
    background: var(--surface-2);
    border-radius: 8px;
    padding: 0.8rem 1rem;
    margin-bottom: 1rem;
  }
  .options {
    display: grid;
    gap: 0.5rem;
  }
  .options button {
    text-align: left;
    font-family: var(--font-read);
    font-size: 1.15rem;
    padding: 0.5em 0.9em;
  }
  kbd {
    font-family: var(--font-ui);
    font-size: 0.75rem;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 0.35em;
    margin-right: 0.3rem;
  }
  .spinner {
    width: 28px;
    height: 28px;
    margin: 0.5rem auto;
    border: 3px solid var(--surface-2);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .headline {
    font-size: 2rem;
    margin: 0;
  }
  .chart {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 0.6rem;
    align-items: end;
    height: 170px;
    margin-top: 1rem;
  }
  .col {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 0.2rem;
    height: 100%;
  }
  .pbar {
    width: 70%;
    background: var(--accent);
    opacity: 0.45;
    border-radius: 4px 4px 0 0;
  }
  .pbar.top {
    opacity: 1;
  }
  .val {
    font-size: 0.8rem;
  }
  .lab {
    font-size: 0.8rem;
    color: var(--muted);
  }
  .by {
    border-collapse: collapse;
    margin-bottom: 0.8rem;
  }
  .by td,
  .by th {
    padding: 0.2rem 0.8rem 0.2rem 0;
    text-align: left;
  }
  .by td.good {
    color: var(--accent);
    font-weight: 600;
  }
  .by td.weak {
    color: var(--muted);
  }
  .hist {
    margin: 0;
    padding-left: 1.2rem;
  }
</style>
