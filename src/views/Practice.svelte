<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    api,
    type PracticeIntro,
    type PracticeItem,
    type PracticeResult,
    type PracticeSheet,
    type PastPractice,
    type ReviewItem,
    type Verdict,
  } from "../lib/api";

  type Phase = "intro" | "preparing" | "taking" | "result" | "review";
  let phase = $state<Phase>("intro");
  let intro = $state<PracticeIntro | null>(null);
  let error = $state("");
  let note = $state("");

  // Intro choices
  let level = $state(5);
  let half = $state(false);
  let timed = $state(true);

  // Preparing
  let testId = $state("");
  let progress = $state({ done: 0, total: 0 });
  let stopping = $state(false);
  const unlisten = listen<{ done: number; total: number }>("practice-progress", (e) => (progress = e.payload));

  // Taking
  let sheet = $state<PracticeSheet | null>(null);
  let partIdx = $state(0);
  let at = $state(0);
  let answers = $state<Record<string, number>>({});
  let used = $state(0);
  let confirmHandIn = $state(false);
  let timesUpShown = $state(false);
  let ticker: ReturnType<typeof setInterval> | null = null;
  let lastSaved = 0;

  // Result and review
  let result = $state<PracticeResult | null>(null);
  let review = $state<ReviewItem[]>([]);
  let missedOnly = $state(true);
  let cardState = $state<Record<string, "making" | "done" | string>>({});

  onMount(loadIntro);
  onDestroy(() => {
    unlisten.then((f) => f());
    stopTicker(true);
  });

  async function loadIntro() {
    try {
      intro = await api.practiceIntro();
      level = intro.suggested;
    } catch (e) {
      error = String(e);
    }
  }

  const layout = $derived(intro?.layouts.find(([l]) => l === level)?.[1] ?? []);
  const fullItems = $derived(layout.reduce((n, p) => n + p.mondai.reduce((m, [, k]) => m + k, 0), 0));
  const halfItems = $derived(layout.reduce((n, p) => n + p.mondai.reduce((m, [, k]) => m + Math.ceil(k / 2), 0), 0));
  const fullMinutes = $derived(layout.reduce((n, p) => n + p.minutes, 0));
  const halfMinutes = $derived(layout.reduce((n, p) => n + Math.ceil(p.minutes / 2), 0));

  // --- preparing ---

  async function create() {
    error = "";
    note = "";
    try {
      testId = await api.practiceCreate(level, half, timed);
      await prepare();
    } catch (e) {
      error = String(e);
    }
  }

  async function prepare() {
    phase = "preparing";
    stopping = false;
    error = "";
    progress = { done: 0, total: 0 };
    try {
      const p = await api.practicePrepare(testId);
      if (p.done < p.total) {
        note = "Stopped. The questions written so far are kept; continue preparing any time.";
        phase = "intro";
        await loadIntro();
        return;
      }
      await openSheet(testId);
    } catch (e) {
      error = String(e);
    }
  }

  async function stop() {
    stopping = true;
    await api.practiceStop();
  }

  // --- taking ---

  async function openSheet(id: string) {
    testId = id;
    error = "";
    const s = await api.practiceSheet(id);
    if (!s.ready) {
      await prepare();
      return;
    }
    if (s.finished) {
      result = await api.practiceFinish(id);
      sheet = s;
      phase = "result";
      return;
    }
    sheet = s;
    answers = { ...s.answers };
    const p = s.parts.findIndex((x) => !x.submitted);
    startPart(p < 0 ? s.parts.length - 1 : p);
    phase = "taking";
  }

  function startPart(p: number) {
    if (!sheet) return;
    stopTicker(false);
    partIdx = p;
    used = sheet.parts[p].used_secs;
    lastSaved = used;
    timesUpShown = limit > 0 && used >= limit;
    confirmHandIn = false;
    const first = flat.findIndex((e) => answers[e.item.id] === undefined);
    at = first < 0 ? 0 : first;
    ticker = setInterval(() => {
      if (document.hidden) return;
      used += 1;
      if (used - lastSaved >= 15) saveTime(false);
    }, 1000);
  }

  function stopTicker(save: boolean) {
    if (ticker) clearInterval(ticker);
    ticker = null;
    if (save && phase === "taking" && testId) saveTime(false);
  }

  function saveTime(submitted: boolean) {
    lastSaved = used;
    return api.practiceTime(testId, partIdx, used, submitted).catch(() => {});
  }

  const part = $derived(sheet?.parts[partIdx]);
  const flat = $derived(
    (part?.groups ?? []).flatMap((g, gi) => g.items.map((item) => ({ item, group: g, gi }))),
  );
  const cur = $derived(flat[at]);
  const limit = $derived(sheet?.timed && part ? part.minutes * 60 : 0);
  const left = $derived(limit - used);
  const unanswered = $derived(flat.filter((e) => answers[e.item.id] === undefined).length);
  /** Items on the same passage as the current one. */
  const siblings = $derived(cur?.item.group ? flat.filter((e) => e.item.group === cur.item.group) : []);
  const isRetry = $derived(sheet?.kind === "retry");
  const curText = $derived(cur ? underlined(cur.item) : ["", "", ""]);

  $effect(() => {
    if (phase === "taking" && limit > 0 && left <= 0 && !timesUpShown) timesUpShown = true;
  });

  function clock(secs: number) {
    const s = Math.abs(secs);
    const m = Math.floor(s / 60);
    return `${secs < 0 ? "+" : ""}${m}:${String(s % 60).padStart(2, "0")}`;
  }

  async function choose(choice: number) {
    if (!cur) return;
    const id = cur.item.id;
    answers[id] = choice;
    try {
      await api.practiceAnswer(testId, id, choice);
    } catch (e) {
      error = String(e);
      return;
    }
    const was = at;
    setTimeout(() => {
      if (at === was && at + 1 < flat.length) at += 1;
    }, 220);
  }

  async function handIn() {
    if (!sheet) return;
    if (unanswered > 0 && !confirmHandIn) {
      confirmHandIn = true;
      return;
    }
    stopTicker(false);
    await saveTime(true);
    sheet.parts[partIdx].submitted = true;
    sheet.parts[partIdx].used_secs = used;
    if (partIdx + 1 < sheet.parts.length) {
      startPart(partIdx + 1);
      return;
    }
    try {
      result = await api.practiceFinish(testId);
      phase = "result";
      await loadIntro();
    } catch (e) {
      error = String(e);
    }
  }

  function leave() {
    stopTicker(true);
    phase = "intro";
    loadIntro();
  }

  function onkey(e: KeyboardEvent) {
    if (phase !== "taking" || !cur) return;
    if ((e.target as HTMLElement)?.tagName === "INPUT") return;
    if (/^[1-4]$/.test(e.key) && Number(e.key) <= cur.item.options.length) choose(Number(e.key) - 1);
    else if (e.key === "ArrowRight" && at + 1 < flat.length) at += 1;
    else if (e.key === "ArrowLeft" && at > 0) at -= 1;
  }

  /** The prompt split around the underlined part. */
  function underlined(item: PracticeItem): [string, string, string] {
    const u = item.underline;
    const i = u ? item.prompt.indexOf(u) : -1;
    if (!u || i < 0) return [item.prompt, "", ""];
    return [item.prompt.slice(0, i), u, item.prompt.slice(i + u.length)];
  }

  /** A 文章の文法 passage split around the current blank. */
  function passageParts(item: PracticeItem, isBlank: boolean): [string, string, string] {
    const p = item.passage ?? "";
    const i = isBlank ? p.indexOf(item.prompt) : -1;
    if (i < 0) return [p, "", ""];
    return [p.slice(0, i), item.prompt, p.slice(i + item.prompt.length)];
  }

  // --- result and review ---

  const VERDICT: Record<Verdict, string> = {
    likely_pass: "Likely pass",
    borderline: "Borderline",
    unlikely: "Not yet",
  };
  const pct = (r: number, t: number) => `${Math.round((100 * r) / Math.max(1, t))}%`;

  async function openResult(t: PastPractice) {
    error = "";
    testId = t.id;
    if (!t.ready) {
      await prepare();
      return;
    }
    if (!t.finished_at) {
      await openSheet(t.id);
      return;
    }
    result = t.result ?? (await api.practiceFinish(t.id));
    phase = "result";
  }

  async function openReview() {
    try {
      review = await api.practiceReview(testId);
      missedOnly = review.some((r) => r.choice !== r.answer);
      phase = "review";
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleReport(r: ReviewItem) {
    try {
      result = await api.practiceReport(testId, r.id, !r.reported);
      r.reported = !r.reported;
    } catch (e) {
      error = String(e);
    }
  }

  async function makeCard(r: ReviewItem) {
    if (!r.word || cardState[r.id] === "making" || cardState[r.id] === "done") return;
    cardState[r.id] = "making";
    try {
      await api.cardCreate({ lang: "ja", lemma: r.word.expression, reading: r.word.reading }, r.word.expression, null, null);
      cardState[r.id] = "done";
    } catch (e) {
      cardState[r.id] = String(e);
    }
  }

  async function makeAllCards() {
    for (const r of missedWords) await makeCard(r);
  }

  async function retry() {
    try {
      const id = await api.practiceRetry(testId);
      await openSheet(id);
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(t: PastPractice) {
    await api.practiceDelete(t.id);
    await loadIntro();
  }

  const missed = $derived(review.filter((r) => r.choice !== r.answer && !r.reported));
  const missedWords = $derived(
    missed.filter((r, i, all) => r.word && all.findIndex((x) => x.word?.expression === r.word?.expression) === i),
  );
  const shown = $derived(missedOnly ? review.filter((r) => r.choice !== r.answer) : review);
  const missedCount = $derived(result ? result.total - result.right : 0);
</script>

<svelte:window onkeydown={onkey} />

<div class="page" class:wide={phase === "taking" || phase === "review"}>
  {#if phase !== "taking"}<h1>Practice test</h1>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if note}<p class="muted">{note}</p>{/if}

  {#if phase === "intro"}
    <p class="muted lead">
      A mock JLPT for one level: the exam's question types in its order and times, without listening. Ankura writes the
      questions before you start (a few minutes), around words from the JLPT lists, and checks each one. You get a
      JLPT-style score, a pass estimate, and every answer explained.
    </p>
    {#if intro}
      <section class="card">
        <h2>New test</h2>
        <div class="row">
          <span class="lab">Level</span>
          {#each [5, 4, 3, 2, 1] as l}
            <button class="seg" class:on={level === l} onclick={() => (level = l)}>N{l}</button>
          {/each}
          {#if level === intro.suggested}<span class="muted small">suggested</span>{/if}
        </div>
        <div class="row">
          <span class="lab">Length</span>
          <button class="seg" class:on={!half} onclick={() => (half = false)}>Full · {fullItems} questions · {fullMinutes} min</button>
          <button class="seg" class:on={half} onclick={() => (half = true)}>Half · {halfItems} questions · {halfMinutes} min</button>
        </div>
        <div class="row">
          <span class="lab">Timer</span>
          <label class="check"><input type="checkbox" bind:checked={timed} /> Official time per part</label>
        </div>
        <ul class="parts muted small">
          {#each layout as p}
            <li><strong>{p.name}</strong> · {half ? Math.ceil(p.minutes / 2) : p.minutes} min</li>
          {/each}
        </ul>
        <button class="primary big" onclick={create}>Prepare the test</button>
      </section>

      {#if intro.tests.length}
        <section class="card">
          <h2>Your tests</h2>
          <ul class="tests">
            {#each intro.tests as t}
              <li>
                <span class="tlvl">N{t.level}</span>
                <span class="tdesc">
                  {#if t.kind === "retry"}
                    Missed questions
                  {:else}
                    {t.half ? "Half" : "Full"} test
                  {/if}
                  <span class="muted small">· {new Date(t.started_at).toLocaleDateString()}</span>
                </span>
                <span class="tstate">
                  {#if !t.ready}
                    <span class="muted">not fully prepared</span>
                  {:else if !t.finished_at}
                    <span class="muted">{t.answered} of {t.items} answered</span>
                  {:else if t.result?.verdict}
                    <span class="verdict v-{t.result.verdict}">{VERDICT[t.result.verdict]}</span>
                    <span class="muted small">{t.result.total_score}/{t.result.total_max}</span>
                  {:else if t.result}
                    <span class="muted">{t.result.right}/{t.result.total} right</span>
                  {/if}
                </span>
                <span class="row">
                  <button onclick={() => openResult(t)}>
                    {!t.ready ? "Continue preparing" : !t.finished_at ? "Continue" : "Results"}
                  </button>
                  <button class="ghost danger" title="Delete this test" onclick={() => remove(t)}>✕</button>
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      <p class="muted small">{intro.credit} Questions are Ankura's own, written by the AI; none are from the JLPT.</p>
    {/if}
  {:else if phase === "preparing"}
    <section class="card center">
      {#if error}
        <p>Preparing stopped with an error. The questions written so far are kept.</p>
        <div class="row center-row">
          <button class="primary" onclick={prepare}>Try again</button>
          <button onclick={leave}>Back</button>
        </div>
      {:else}
        <div class="spinner" aria-hidden="true"></div>
        <p>Writing and checking the questions…</p>
        {#if progress.total}
          <div class="progress"><div style:width="{(100 * progress.done) / progress.total}%"></div></div>
          <p class="muted small">{progress.done} of {progress.total} question types ready</p>
        {/if}
        <p class="muted small">This takes a few minutes. You can stop and continue later.</p>
        <button onclick={stop} disabled={stopping}>{stopping ? "Stopping after this round…" : "Stop"}</button>
      {/if}
    </section>
  {:else if phase === "taking" && sheet && part && cur}
    <header class="bar">
      <div>
        <strong>{isRetry ? "Missed questions" : `N${sheet.level} · ${part.name}`}</strong>
        {#if sheet.parts.length > 1}<span class="muted small"> · part {partIdx + 1} of {sheet.parts.length}</span>{/if}
      </div>
      <div class="row">
        {#if limit > 0}
          <span class="clock" class:low={left < 300 && left > 0} class:over={left <= 0}>{clock(left)}</span>
        {:else}
          <span class="clock muted">{clock(used)}</span>
        {/if}
        <button onclick={leave} title="Your answers and time are saved">Pause</button>
        <button class="primary" onclick={handIn}>{partIdx + 1 < sheet.parts.length ? "Hand in this part" : "Hand in"}</button>
      </div>
    </header>
    {#if timesUpShown && left <= 0}
      <p class="banner">Time's up for this part. Hand it in, or keep going: the result will mark it as overtime.</p>
    {/if}
    {#if confirmHandIn}
      <p class="banner">
        {unanswered} question{unanswered === 1 ? "" : "s"} not answered (they count as wrong).
        <button class="primary" onclick={handIn}>Hand in anyway</button>
        <button onclick={() => (confirmHandIn = false)}>Keep working</button>
      </p>
    {/if}

    <nav class="grid" aria-label="Questions">
      {#each flat as e, i}
        {#if i === 0 || flat[i - 1].gi !== e.gi}<span class="gsep" title={e.group.name_ja}>問題{e.gi + 1}</span>{/if}
        <button class="qn" class:done={answers[e.item.id] !== undefined} class:now={i === at} onclick={() => (at = i)}>
          {e.item.number}
        </button>
      {/each}
    </nav>

    <section class="card instr">
      <p><strong>問題{cur.gi + 1}</strong> <span class="muted small">{cur.group.name_ja}</span></p>
      <p class="ja">{cur.group.instruction}</p>
      <p class="muted small">{cur.group.hint}</p>
    </section>

    <div class="qarea" class:split={!!cur.item.passage}>
      {#if cur.item.passage}
        {@const isBlank = cur.group.mondai === "text_grammar"}
        {@const [pa, pb, pc] = passageParts(cur.item, isBlank)}
        <section class="card passage">
          {#if siblings.length > 1}
            <p class="muted small">Questions {siblings[0].item.number}-{siblings[siblings.length - 1].item.number} are about this text.</p>
          {/if}
          <div class="ptext">{pa}{#if pb}<mark>{pb}</mark>{/if}{pc}</div>
        </section>
      {/if}
      <section class="card q">
        <p class="sentence"><span class="num">{cur.item.number}</span>{curText[0]}{#if curText[1]}<u>{curText[1]}</u>{/if}{curText[2]}</p>
        <div class="options" class:long={cur.group.mondai === "usage"}>
          {#each cur.item.options as o, i}
            <button class:picked={answers[cur.item.id] === i} onclick={() => choose(i)}><kbd>{i + 1}</kbd> {o}</button>
          {/each}
        </div>
        <div class="row nav">
          <button disabled={at === 0} onclick={() => (at -= 1)}>← Back</button>
          <span class="muted small">{flat.length - unanswered} of {flat.length} answered</span>
          <button disabled={at + 1 >= flat.length} onclick={() => (at += 1)}>Next →</button>
        </div>
      </section>
    </div>
  {:else if phase === "result" && result}
    <section class="card">
      {#if result.verdict}
        <p class="muted small">N{result.level} practice test{result.half ? " (half length)" : ""}</p>
        <p class="headline"><span class="verdict big v-{result.verdict}">{VERDICT[result.verdict]}</span></p>
        <p>
          <strong>{result.total_score}</strong> of {result.total_max} points without listening; the pass line for that is about
          <strong>{result.pass_line}</strong>.
        </p>
        <div class="sections">
          {#each result.sections as s}
            <div class="sec">
              <div class="row between"><span class="ja">{s.name}</span><strong>{s.score}/{s.max}</strong></div>
              <div class="sbar">
                <div class="fill" class:under={s.score < s.minimum} style:width="{(100 * s.score) / s.max}%"></div>
                <div class="tick" style:left="{(100 * s.minimum) / s.max}%" title="Sectional minimum {s.minimum}"></div>
              </div>
              <p class="muted small">{s.right} of {s.total} right · minimum {s.minimum}{s.score < s.minimum ? " — below it, which fails the test on its own" : ""}</p>
            </div>
          {/each}
        </div>
        {#if result.grades.length}
          <div class="row grades">
            {#each result.grades as g}
              <span class="grade g-{g.grade}" title="{g.right} of {g.total} right">{g.name} <strong>{g.grade}</strong></span>
            {/each}
            <span class="muted small">A: 67% or more · B: 34-67% · C: under 34%, as on the JLPT score report</span>
          </div>
        {/if}
      {:else}
        <p class="muted small">Missed questions, again</p>
        <p class="headline"><strong>{result.right} of {result.total}</strong> <span class="muted">right this time</span></p>
      {/if}
      {#if result.overtime.length}
        <p class="muted">Over time: {result.overtime.join(", ")}.</p>
      {/if}
      {#if result.unanswered}<p class="muted">{result.unanswered} not answered (counted as wrong).</p>{/if}
      {#if result.reported}<p class="muted">{result.reported} reported question{result.reported === 1 ? "" : "s"} left out of the score.</p>{/if}
    </section>

    <section class="card">
      <h2>By question type</h2>
      <table class="by">
        <tbody>
          {#each result.by_type as t}
            <tr>
              <td class="ja">{t.name_ja}</td>
              <td>{t.right}/{t.total}</td>
              <td class="barcell"><div class="mini"><div style:width={pct(t.right, t.total)}></div></div></td>
            </tr>
          {/each}
        </tbody>
      </table>
      <div class="row">
        <button class="primary" onclick={openReview}>Review answers</button>
        {#if missedCount > 0}<button onclick={retry}>Retry the {missedCount} missed</button>{/if}
        <button onclick={leave}>Done</button>
      </div>
      {#if result.verdict}
        <p class="muted small">
          An estimate, not a JLPT score: the real exam scales scores by question difficulty, which Ankura can't know, and
          includes listening. The pass line here is the official one scaled to the sections tested. AI-written questions can
          be easier or harder than the real ones; report any that are wrong.
        </p>
      {/if}
    </section>
  {:else if phase === "review"}
    <header class="bar">
      <div class="row">
        <button onclick={() => (phase = "result")}>← Results</button>
        <label class="check"><input type="checkbox" bind:checked={missedOnly} /> Missed only</label>
      </div>
      {#if missedWords.length}
        <button onclick={makeAllCards}>Anki cards for all {missedWords.length} missed words</button>
      {/if}
    </header>
    {#if !shown.length}<p class="muted">Nothing missed. Well done.</p>{/if}
    {#each shown as r, i (r.id)}
      {#if r.passage && (i === 0 || shown[i - 1].group !== r.group)}
        <details class="card passage" open={r.passage.length < 400}>
          <summary class="muted small">The text ({r.name_ja})</summary>
          <div class="ptext">{r.passage}</div>
        </details>
      {/if}
      {@const [a, u, b] = underlined(r)}
      <section class="card rev" class:reported={r.reported}>
        <p class="muted small">{r.number} · <span class="ja">{r.name_ja}</span></p>
        <p class="sentence">{a}{#if u}<u>{u}</u>{/if}{b}</p>
        <ol class="ropts">
          {#each r.options as o, j}
            <li class:key={j === r.answer} class:wrong={j === r.choice && j !== r.answer}>
              {o}
              {#if j === r.answer}<span class="tag">answer</span>{/if}
              {#if j === r.choice && j !== r.answer}<span class="tag">yours</span>{/if}
            </li>
          {/each}
        </ol>
        {#if r.choice === null}<p class="muted small">Not answered.</p>{/if}
        {#if r.explanation}<p class="expl">{r.explanation}</p>{/if}
        <div class="row">
          <button class="ghost small" onclick={() => toggleReport(r)}>
            {r.reported ? "Reported · undo" : "Report a bad question"}
          </button>
          {#if r.word && r.choice !== r.answer}
            {@const st = cardState[r.id]}
            <button class="small" disabled={st === "making" || st === "done"} onclick={() => makeCard(r)}>
              {st === "making" ? "Making card…" : st === "done" ? "Card drafted ✓" : `Anki card for ${r.word.expression}`}
            </button>
            {#if st && st !== "making" && st !== "done"}<span class="error small">{st}</span>{/if}
          {/if}
        </div>
      </section>
    {/each}
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
  .page.wide {
    max-width: 1180px;
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
  .lab {
    width: 4rem;
    color: var(--muted);
  }
  section .row + .row {
    margin-top: 0.6rem;
  }
  .seg.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .check {
    display: inline-flex;
    gap: 0.4rem;
    align-items: center;
  }
  .parts {
    margin: 0.8rem 0 0;
    padding-left: 1.2rem;
  }
  .big {
    font-size: 1.05rem;
    padding: 0.6em 1.6em;
    margin-top: 0.8rem;
  }
  .center {
    text-align: center;
  }
  .center-row {
    justify-content: center;
  }
  .tests {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tests li {
    display: grid;
    grid-template-columns: 2.6rem 1fr auto auto;
    gap: 0.8rem;
    align-items: center;
    padding: 0.4rem 0;
    border-top: 1px solid var(--line);
  }
  .tests li:first-child {
    border-top: none;
  }
  .tlvl {
    font-weight: 700;
  }
  .verdict {
    display: inline-block;
    padding: 0.1em 0.7em;
    border-radius: 999px;
    font-weight: 600;
    font-size: 0.85rem;
  }
  .verdict.big {
    font-size: 1.6rem;
    padding: 0.15em 0.8em;
  }
  .v-likely_pass {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .v-borderline {
    background: var(--gold);
    color: #2a2116;
  }
  .v-unlikely {
    background: var(--surface-2);
    color: var(--danger);
    border: 1px solid var(--danger);
  }
  .progress,
  .sbar,
  .mini {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-2);
    overflow: hidden;
    position: relative;
  }
  .progress {
    max-width: 420px;
    margin: 0.6rem auto 0.2rem;
  }
  .progress div,
  .mini div,
  .sbar .fill {
    height: 100%;
    background: var(--accent);
  }
  .sbar {
    height: 12px;
    overflow: visible;
  }
  .sbar .fill {
    border-radius: 6px;
  }
  .sbar .fill.under {
    background: var(--danger);
  }
  .sbar .tick {
    position: absolute;
    top: -3px;
    width: 2px;
    height: 18px;
    background: var(--text);
    opacity: 0.6;
  }
  .sections {
    display: grid;
    gap: 0.9rem;
    margin: 1rem 0;
  }
  .between {
    justify-content: space-between;
  }
  .grades {
    margin-top: 0.4rem;
  }
  .grade {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.2em 0.7em;
    font-family: var(--font-read);
  }
  .grade.g-A strong {
    color: var(--accent);
  }
  .grade.g-C strong {
    color: var(--danger);
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
    margin: 0.2rem 0 0.6rem;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--bg);
    padding: 0.4rem 0;
  }
  .clock {
    font-variant-numeric: tabular-nums;
    font-size: 1.3rem;
    font-weight: 600;
    min-width: 4.5rem;
    text-align: right;
  }
  .clock.low {
    color: var(--gold);
  }
  .clock.over {
    color: var(--danger);
  }
  .banner {
    background: var(--surface-2);
    border-left: 4px solid var(--gold);
    padding: 0.6rem 0.9rem;
    border-radius: 6px;
    margin: 0;
  }
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    align-items: center;
  }
  .gsep {
    font-size: 0.72rem;
    color: var(--muted);
    margin: 0 0.15rem 0 0.5rem;
    font-family: var(--font-read);
  }
  .qn {
    min-width: 2.1rem;
    padding: 0.15em 0.3em;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .qn.done {
    background: var(--surface-2);
    color: var(--muted);
  }
  .qn.now {
    border-color: var(--accent);
    outline: 2px solid var(--accent);
    color: var(--text);
  }
  .instr p {
    margin: 0.15rem 0;
  }
  .qarea {
    display: grid;
    gap: 1rem;
  }
  .qarea.split {
    grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
    align-items: start;
  }
  @media (max-width: 900px) {
    .qarea.split {
      grid-template-columns: 1fr;
    }
  }
  .passage .ptext {
    font-family: var(--font-read);
    font-size: 1.1rem;
    line-height: 1.95;
    white-space: pre-line;
  }
  .qarea .passage {
    max-height: 70vh;
    overflow: auto;
    position: sticky;
    top: 3.5rem;
  }
  mark {
    background: var(--st-learning-bg);
    color: inherit;
    border-bottom: 2px solid var(--gold);
    padding: 0 0.1em;
  }
  .q {
    padding: 1.4rem;
  }
  .sentence {
    font-family: var(--font-read);
    font-size: 1.35rem;
    line-height: 1.9;
    margin-top: 0;
  }
  .sentence u {
    text-decoration-thickness: 2px;
    text-underline-offset: 5px;
  }
  .num {
    display: inline-block;
    font-family: var(--font-ui);
    font-size: 0.8rem;
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 0.4em;
    margin-right: 0.6rem;
    vertical-align: middle;
    color: var(--muted);
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .options.long {
    grid-template-columns: 1fr;
  }
  .options button {
    text-align: left;
    font-family: var(--font-read);
    font-size: 1.1rem;
    padding: 0.5em 0.9em;
  }
  .options button.picked {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .options button.picked kbd {
    color: inherit;
    border-color: currentColor;
  }
  .nav {
    justify-content: space-between;
    margin-top: 1rem;
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
  .by {
    border-collapse: collapse;
    margin-bottom: 0.8rem;
    width: 100%;
  }
  .by td {
    padding: 0.2rem 0.8rem 0.2rem 0;
  }
  .barcell {
    width: 45%;
  }
  .rev.reported {
    opacity: 0.55;
  }
  .rev p {
    margin: 0.2rem 0;
  }
  .ropts {
    font-family: var(--font-read);
    font-size: 1.05rem;
    margin: 0.4rem 0;
  }
  .ropts li {
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
  }
  .ropts li.key {
    color: var(--accent);
    font-weight: 600;
  }
  .ropts li.wrong {
    color: var(--danger);
    text-decoration: line-through;
  }
  .tag {
    font-family: var(--font-ui);
    font-size: 0.7rem;
    border: 1px solid currentColor;
    border-radius: 999px;
    padding: 0 0.5em;
    margin-left: 0.4rem;
    text-decoration: none;
    display: inline-block;
  }
  .expl {
    white-space: pre-line;
    background: var(--surface-2);
    border-radius: 6px;
    padding: 0.5rem 0.8rem;
  }
  details.passage summary {
    cursor: pointer;
  }
  details.passage .ptext {
    margin-top: 0.5rem;
  }
</style>
