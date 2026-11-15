<script lang="ts">
  // Review (ADR 0022): short rounds on what you're about to forget. Fill the gap, listen and
  // fill, speed match, kanji. Ankura keeps its own schedule; a word recalled without help is also
  // answered "Good" on its Anki card.
  import { onDestroy, onMount } from "svelte";
  import {
    api,
    keyId,
    type ReviewAnswer,
    type RoundItem,
    type ReviewMode,
    type ReviewOutcome,
    type ReviewOverview,
    type ReviewSendReport,
  } from "../lib/api";
  import * as speech from "../lib/speech";
  import { app } from "../lib/lang.svelte";

  let { ongo }: { ongo: (view: "anki" | "guide" | "cards") => void } = $props();

  const MODES: { mode: ReviewMode; title: string; text: string }[] = [
    { mode: "gap", title: "Fill the gap", text: "A sentence you read, one word missing. Type it. Recalled words also count in Anki." },
    { mode: "listen", title: "Listen and fill", text: "Hear the sentence, type the missing word." },
    { mode: "match", title: "Speed match", text: "Pair words with their meanings, against the clock." },
    { mode: "kanji", title: "Kanji round", text: "Kanji and keyword, both ways, with your story after each." },
  ];
  const BOARD = 5;

  let ov = $state<ReviewOverview | null>(null);
  let error = $state("");
  let note = $state("");
  let phase = $state<"home" | "loading" | "play" | "done">("home");
  let mode = $state<ReviewMode>("gap");
  let items = $state<RoundItem[]>([]);
  let at = $state(0);
  let typed = $state("");
  let outcome = $state<ReviewOutcome | null>(null);
  let hinted = $state(false);
  let showChoices = $state(false);
  let chosen = $state<number | null>(null);
  let busy = $state(false);
  let combo = $state(0);
  let bestCombo = $state(0);
  let score = $state(0);
  let results = $state<{ item: RoundItem; outcome: ReviewOutcome }[]>([]);
  let sent = $state<ReviewSendReport | null>(null);
  let carded = $state<string[]>([]);
  let voiceError = $state("");
  let input: HTMLInputElement | undefined = $state();

  // Speed match: a board of up to five words and their meanings.
  let boardStart = $state(0);
  let glossOrder = $state<number[]>([]);
  let picked = $state<number | null>(null);
  let matched = $state<number[]>([]);
  let slipped = $state<number[]>([]);
  let flash = $state<number | null>(null);
  let seconds = $state(0);
  let timer: ReturnType<typeof setInterval> | undefined;

  const item = $derived(items[at]);
  const board = $derived(items.slice(boardStart, boardStart + BOARD));
  const right = $derived(results.filter((r) => r.outcome.right).length);
  const hasSentence = (i: RoundItem) => !!(i.before || i.after);

  async function load() {
    try {
      ov = await api.reviewOverview();
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);
  onDestroy(() => {
    speech.stop();
    clearInterval(timer);
  });

  async function start(m: ReviewMode, extra = false) {
    error = "";
    note = "";
    mode = m;
    phase = "loading";
    try {
      items = await api.reviewRound(m, extra);
    } catch (e) {
      error = String(e);
      phase = "home";
      return;
    }
    if (!items.length) {
      phase = "home";
      note = extra
        ? "Nothing to practise in this round yet. Read a text and look words up, or sync Anki."
        : m === "listen"
          ? "No due word has a sentence to listen to yet."
          : "Nothing is due in this round.";
      return;
    }
    at = 0;
    combo = 0;
    bestCombo = 0;
    score = 0;
    results = [];
    sent = null;
    carded = [];
    phase = "play";
    if (m === "match") {
      seconds = 0;
      clearInterval(timer);
      timer = setInterval(() => (seconds += 1), 1000);
      deal(0);
    } else {
      show();
    }
  }

  /** Sets up the current item. */
  function show() {
    typed = "";
    outcome = null;
    hinted = false;
    showChoices = false;
    chosen = null;
    voiceError = "";
    if (mode === "listen") play();
    setTimeout(() => input?.focus(), 0);
  }

  async function play() {
    if (!item) return;
    try {
      await speech.say(item.before + item.answer + item.after);
    } catch (e) {
      voiceError = String(e);
    }
  }

  async function submit(it: RoundItem, a: Partial<ReviewAnswer>): Promise<ReviewOutcome | null> {
    busy = true;
    try {
      const o = await api.reviewAnswer({ kind: it.kind, key: it.key, display: it.display, mode, combo, ...a });
      combo = o.right ? combo + 1 : 0;
      bestCombo = Math.max(bestCombo, combo);
      score += o.points;
      results.push({ item: it, outcome: o });
      return o;
    } catch (e) {
      error = String(e);
      return null;
    } finally {
      busy = false;
    }
  }

  async function check() {
    if (!item || outcome || busy || !typed.trim()) return;
    outcome = await submit(item, { typed, accept: item.accept, helped: hinted });
  }
  async function choose(i: number) {
    if (!item || outcome || busy) return;
    chosen = i;
    outcome = await submit(item, { right: i === item.option_answer });
  }
  async function giveUp() {
    if (!item || outcome || busy) return;
    outcome = await submit(item, { right: false });
  }

  async function next() {
    speech.stop();
    if (at + 1 < items.length) {
      at += 1;
      show();
    } else {
      await finish();
    }
  }

  async function finish() {
    clearInterval(timer);
    phase = "done";
    // "Good" for the Anki cards of the words recalled, and anything still waiting.
    if (results.some((r) => r.outcome.anki) || (ov?.waiting ?? 0) > 0) {
      try {
        sent = await api.reviewSend();
      } catch (e) {
        error = String(e);
      }
    }
    await load();
  }

  // --- speed match ---

  function deal(from: number) {
    boardStart = from;
    const n = Math.min(BOARD, items.length - from);
    const order = [...Array(n).keys()];
    for (let i = n - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [order[i], order[j]] = [order[j], order[i]];
    }
    glossOrder = order;
    picked = null;
    matched = [];
    slipped = [];
  }

  async function pickGloss(g: number) {
    if (picked === null || busy || matched.includes(g)) return;
    const w = picked;
    // Two words can share a meaning; either pairing is right.
    if (g === w || board[g].gloss === board[w].gloss) {
      picked = null;
      matched = [...matched, w];
      await submit(board[w], { right: !slipped.includes(w) });
      if (matched.length >= board.length) {
        if (boardStart + BOARD < items.length) deal(boardStart + BOARD);
        else await finish();
      }
    } else {
      if (!slipped.includes(w)) slipped = [...slipped, w];
      flash = g;
      setTimeout(() => (flash = null), 400);
    }
  }

  // --- after a round ---

  async function makeCard(it: RoundItem) {
    error = "";
    try {
      await api.cardCreate(it.key, it.display, hasSentence(it) ? it.before + it.answer + it.after : null, null);
      carded = [...carded, keyId(it.key)];
    } catch (e) {
      error = String(e);
    }
  }

  async function setToAnki(on: boolean) {
    await api.setSetting("review_anki", on ? "on" : "off");
    await load();
  }
  async function setGoal(goal: number) {
    await api.setSetting("review_goal", String(goal));
    await load();
  }
  async function sendNow() {
    try {
      sent = await api.reviewSend();
    } catch (e) {
      error = String(e);
    }
    await load();
  }

  function days(d: number): string {
    if (d < 1) return `${Math.max(1, Math.round(d * 24))} h`;
    if (d < 45) return `${Math.round(d)} ${Math.round(d) === 1 ? "day" : "days"}`;
    if (d < 365) return `${Math.round(d / 30)} months`;
    return `${(d / 365).toFixed(1)} years`;
  }
  const STATUS: Record<string, string> = {
    pending: "waiting",
    sent: "answered Good",
    not_due: "not due in Anki",
    failed: "failed",
    expired: "not sent (Anki was off for a day)",
  };
  const RESULT: Record<string, string> = { recalled: "Recalled", helped: "Right, with help", missed: "Missed" };

  function onkey(e: KeyboardEvent) {
    if (phase !== "play" || mode === "match" || e.isComposing) return;
    if (outcome) {
      if (e.key === "Enter") {
        e.preventDefault();
        next();
      }
      return;
    }
    const picking = mode === "kanji" || showChoices;
    if (picking && item && /^[1-4]$/.test(e.key) && Number(e.key) <= item.options.length) choose(Number(e.key) - 1);
    else if (!picking && e.key === "Enter") check();
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="page review">
  <h1>Review</h1>
  {#if error}<p class="error">{error}</p>{/if}

  {#if phase === "home"}
    {#if ov}
      <p class="muted lead">
        Short rounds on what you're about to forget: fading Anki words, words you looked up, and kanji.
        <button class="link" onclick={() => ongo("guide")}>How it works</button>
      </p>
      <div class="tiles">
        <div class="tile">
          <span class="label">Due now</span><span class="value">{ov.due_words}</span>
          <span class="sub">{ov.due_fading} fading in Anki{app.lang === "ja" ? ` · ${ov.due_kanji} kanji` : ""}</span>
        </div>
        <div class="tile">
          <span class="label">Today</span><span class="value">{ov.today_right} / {ov.goal}</span>
          <span class="bar"><span style:width="{Math.min(100, (ov.today_right / ov.goal) * 100)}%"></span></span>
          <span class="sub">{ov.today_points} points</span>
        </div>
        <div class="tile">
          <span class="label">Review streak</span><span class="value">{ov.streak} {ov.streak === 1 ? "day" : "days"}</span>
          <span class="sub">best {ov.best_streak}</span>
        </div>
        <div class="tile">
          <span class="label">Rescued in Anki</span><span class="value">{ov.rescued_week}</span>
          <span class="sub">cards this week</span>
        </div>
      </div>

      {#if note}<p class="note">{note}</p>{/if}

      <div class="modes">
        {#each MODES.filter((m) => m.mode !== "kanji" || app.lang === "ja") as m}
          {@const due = m.mode === "kanji" ? ov.due_kanji : ov.due_words}
          <section class="card mode" class:main={m.mode === "gap"}>
            <h2>{m.title}</h2>
            <p class="muted small">{m.text}</p>
            <div class="row">
              <button class={m.mode === "gap" ? "primary" : ""} disabled={due === 0} onclick={() => start(m.mode)}>
                {due === 0 ? "Nothing due" : "Start"}
              </button>
              {#if due === 0}<button class="ghost" onclick={() => start(m.mode, true)}>Practise anyway</button>{/if}
            </div>
          </section>
        {/each}
      </div>

      <section class="card">
        <h2>Anki</h2>
        <label>
          <input type="checkbox" checked={ov.to_anki} onchange={(e) => setToAnki(e.currentTarget.checked)} />
          When I recall a word in Fill the gap without help, answer its Anki card with “Good”
        </label>
        <p class="muted small">
          Only cards that are due in Anki, and only for typed answers without a hint. Anki's own scheduler sets the next
          date. Wrong answers change nothing in Anki. Anki has to be running; answers wait up to a day.
        </p>
        {#if ov.waiting > 0}
          <p>
            {ov.waiting} {ov.waiting === 1 ? "answer is" : "answers are"} waiting for Anki.
            <button onclick={sendNow}>Send now</button>
          </p>
        {/if}
        {#if sent?.error}<p class="muted small">{sent.error}</p>{/if}
        {#if ov.anki_log.length}
          <table class="log">
            <tbody>
              {#each ov.anki_log.slice(0, 8) as a}
                <tr>
                  <td class="ja">{a.display}</td>
                  <td>{STATUS[a.status] ?? a.status}</td>
                  <td class="muted">{new Date(a.ts).toLocaleString()}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
        <p class="row small">
          <span class="muted">Daily goal:</span>
          <select value={ov.goal} onchange={(e) => setGoal(Number(e.currentTarget.value))} aria-label="Daily goal">
            {#each [...new Set([10, 20, 30, 50, ov.goal])].sort((a, b) => a - b) as g}
              <option value={g}>{g} right answers</option>
            {/each}
          </select>
          <span class="muted">· {ov.tracked} words and kanji on Ankura's schedule</span>
        </p>
      </section>
    {:else if !error}
      <p class="muted">Looking at what's due…</p>
    {/if}
  {:else if phase === "loading"}
    <p class="muted">Getting the round ready…</p>
  {:else if phase === "play"}
    <div class="hud">
      <span class="pill">{MODES.find((m) => m.mode === mode)?.title}</span>
      {#if mode === "match"}
        <span>{results.length} / {items.length}</span>
        <span class="clock">{Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, "0")}</span>
      {:else}
        <span>{at + 1} / {items.length}</span>
      {/if}
      <span class="grow"></span>
      {#if combo >= 2}<span class="combo">{combo} in a row</span>{/if}
      <span class="score">{score} points</span>
      <button class="ghost" onclick={finish}>End round</button>
    </div>

    {#if mode === "match"}
      <section class="card match">
        <p class="muted small">Pick a word, then its meaning.</p>
        <div class="board">
          <div class="col">
            {#each board as b, i}
              <button class="ja" class:picked={picked === i} class:gone={matched.includes(i)} disabled={matched.includes(i)} onclick={() => (picked = i)}>
                {b.display}
              </button>
            {/each}
          </div>
          <div class="col">
            {#each glossOrder as g}
              <button class:gone={matched.includes(g)} class:wrong={flash === g} disabled={matched.includes(g) || picked === null} onclick={() => pickGloss(g)}>
                {board[g]?.gloss}
              </button>
            {/each}
          </div>
        </div>
      </section>
    {:else if item}
      <section class="card item">
        {#if item.kind === "kanji"}
          <p class="muted small">{item.ask === "keyword" ? "Which keyword?" : "Which kanji?"}</p>
          <p class="prompt" class:big={item.ask === "keyword"}>{item.ask === "keyword" ? item.display : item.gloss}</p>
        {:else}
          {#if mode === "listen"}
            <p class="row">
              <button onclick={play}>▶ Play again</button>
              {#if voiceError}<span class="error">{voiceError}</span>{/if}
            </p>
          {/if}
          {#if hasSentence(item)}
            <p class="sentence">
              <span class="gap-before">{item.before}</span>{#if outcome}<span class="filled" class:missed={!outcome.right}>{item.answer}</span>{:else}<span class="gap" style:min-width="{Math.max(2, item.answer.length) * 1.4}em">{typed}</span>{/if}<span class="gap-after">{item.after}</span>
            </p>
          {:else}
            <p class="muted small">Which word is this?</p>
          {/if}
          {#if item.gloss && (mode !== "listen" || outcome)}<p class="cue">{item.gloss}</p>{/if}
        {/if}

        {#if !outcome}
          {#if item.kind === "kanji" || showChoices}
            <div class="options" class:kanji={item.ask === "kanji"}>
              {#each item.options as o, i}
                <button class:ja={item.kind === "word" || item.ask === "kanji"} disabled={busy} onclick={() => choose(i)}><kbd>{i + 1}</kbd> {o}</button>
              {/each}
            </div>
          {:else}
            <div class="row">
              <input
                class="answer"
                bind:this={input}
                bind:value={typed}
                placeholder={app.lang === "ja" ? "the missing word: kanji, kana or romaji" : "the missing word"}
                autocomplete="off"
                autocapitalize="off"
                spellcheck="false"
              />
              <button class="primary" disabled={busy || !typed.trim()} onclick={check}>Check</button>
            </div>
            {#if hinted}
              {@const spelled = [...(item.answer_reading || item.answer)]}
              <p class="muted small hint">Starts with <strong class="ja">{spelled[0]}</strong>, {spelled.length} {item.answer_reading ? "kana" : "letters"}.</p>
            {/if}
            <div class="row helpers">
              {#if !hinted}<button class="ghost" onclick={() => (hinted = true)}>Hint</button>{/if}
              {#if item.options.length}<button class="ghost" onclick={() => (showChoices = true)}>Show choices</button>{/if}
              <button class="ghost" onclick={giveUp}>I don't know</button>
            </div>
          {/if}
        {:else}
          <div class="reveal" class:missed={!outcome.right}>
            <p>
              <strong class="ja word">{item.display}</strong>
              {#if item.reading && item.reading !== item.display}<span class="ja muted">{item.reading}</span>{/if}
              {#if item.kind === "kanji"}<span>{item.gloss}</span>{/if}
            </p>
            {#if item.kind === "kanji" && item.story}<p class="story">{item.story}</p>{/if}
            <p class="small">
              <strong>{RESULT[outcome.result]}</strong>{#if outcome.points}&nbsp;· +{outcome.points}{/if}
              · back in {days(outcome.next_days)}
              {#if outcome.anki}<span class="pill anki">Anki: Good</span>{/if}
            </p>
            {#if item.from}<p class="muted small">From: {item.from}</p>{/if}
          </div>
          <button class="primary" onclick={next}>{at + 1 < items.length ? "Next" : "See the result"}</button>
        {/if}
      </section>
    {/if}
  {:else if phase === "done"}
    <section class="card result">
      <p class="total"><strong>{score}</strong> points</p>
      <p>
        {right} of {results.length} right{#if bestCombo >= 2}&nbsp;· best run {bestCombo} in a row{/if}{#if mode === "match"}&nbsp;· {seconds} s{/if}
      </p>
      {#if ov}
        <p class="muted">
          Today: {ov.today_right} / {ov.goal} right answers{ov.today_right >= ov.goal ? ". Goal met." : "."}
          {#if ov.streak > 0}Streak: {ov.streak} {ov.streak === 1 ? "day" : "days"}.{/if}
        </p>
      {/if}
      {#if sent}
        <p class="anki-report">
          {#if sent.sent}Anki: {sent.sent} {sent.sent === 1 ? "card" : "cards"} answered “Good”.{/if}
          {#if sent.not_due}{sent.not_due} not due there, left alone.{/if}
          {#if sent.failed}{sent.failed} failed.{/if}
          {#if sent.waiting}{sent.waiting} waiting. {sent.error ?? ""}{/if}
        </p>
      {/if}
      {#if results.some((r) => !r.outcome.right)}
        <h2>Missed</h2>
        <ul class="words">
          {#each results.filter((r) => !r.outcome.right) as r}
            <li><span class="ja">{r.item.display}</span> <span class="muted">{r.item.gloss}</span></li>
          {/each}
        </ul>
        <p class="muted small">They come back within a day.</p>
      {/if}
      {#if results.some((r) => r.item.kind === "word" && !r.item.has_card)}
        <h2>Not in Anki yet</h2>
        <ul class="words">
          {#each results.filter((r, i, all) => r.item.kind === "word" && !r.item.has_card && all.findIndex((x) => keyId(x.item.key) === keyId(r.item.key)) === i) as r}
            <li>
              <span class="ja">{r.item.display}</span> <span class="muted">{r.item.gloss}</span>
              {#if carded.includes(keyId(r.item.key))}
                <span class="pill">card drafted</span>
              {:else}
                <button class="ghost" onclick={() => makeCard(r.item)}>+ card</button>
              {/if}
            </li>
          {/each}
        </ul>
        {#if carded.length}<p class="small"><button class="link" onclick={() => ongo("cards")}>Open Cards</button> to check and add them.</p>{/if}
      {/if}
      <div class="row">
        <button class="primary" onclick={() => start(mode)}>Another round</button>
        <button onclick={() => (phase = "home")}>Back to Review</button>
      </div>
    </section>
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
  }
  .link {
    border: none;
    background: none;
    color: var(--accent);
    padding: 0;
    text-decoration: underline;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 0.8rem;
  }
  .tile {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 0.8rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .tile .label,
  .tile .sub {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .tile .value {
    font-size: 1.5rem;
    font-weight: 700;
  }
  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .note {
    background: var(--surface-2);
    border-radius: 8px;
    padding: 0.5rem 0.8rem;
    margin: 0;
  }
  .modes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }
  .mode.main {
    border-color: var(--accent);
  }
  .mode p {
    margin: 0 0 0.7rem;
    min-height: 2.4em;
  }
  .log {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
    margin: 0.4rem 0;
  }
  .log td {
    padding: 0.2rem 0.6rem 0.2rem 0;
    border-top: 1px solid var(--line);
  }
  .hud {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .grow {
    flex: 1;
  }
  .combo {
    color: var(--gold);
    font-weight: 700;
  }
  .score {
    font-weight: 700;
  }
  .clock {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .item p {
    margin: 0;
  }
  .sentence {
    font-family: var(--font-read);
    font-size: 1.5rem;
    line-height: 2;
  }
  .gap {
    display: inline-block;
    border-bottom: 2px solid var(--accent);
    text-align: center;
    margin: 0 0.15em;
    min-height: 1.4em;
    vertical-align: bottom;
    color: var(--accent);
  }
  .filled {
    color: var(--accent);
    font-weight: 700;
    border-bottom: 2px solid var(--accent);
    margin: 0 0.1em;
  }
  .filled.missed {
    color: var(--danger);
    border-color: var(--danger);
  }
  .cue {
    color: var(--muted);
    font-size: 1.05rem;
  }
  .prompt {
    font-size: 1.4rem;
  }
  .prompt.big {
    font-family: var(--font-read);
    font-size: 4rem;
    line-height: 1.2;
  }
  .answer {
    flex: 1;
    font-size: 1.15rem;
    font-family: var(--font-read);
  }
  .helpers {
    gap: 0.3rem;
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .options button {
    text-align: left;
    font-size: 1.05rem;
  }
  .options.kanji button {
    font-size: 1.8rem;
  }
  .reveal {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    border-radius: 8px;
    padding: 0.6rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .reveal.missed {
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }
  .reveal .word {
    font-size: 1.4rem;
    margin-right: 0.5rem;
  }
  .story {
    white-space: pre-wrap;
  }
  .pill.anki {
    background: var(--accent);
    color: var(--accent-ink);
    margin-left: 0.4rem;
  }
  .item > button {
    align-self: flex-start;
  }
  .board {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 0.8rem;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .col button {
    text-align: left;
    min-height: 2.8rem;
  }
  .col button.ja {
    font-size: 1.2rem;
  }
  .col button.picked {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .col button.gone {
    visibility: hidden;
  }
  .col button.wrong {
    border-color: var(--danger);
    color: var(--danger);
  }
  .result {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .result p {
    margin: 0;
  }
  .total {
    font-size: 1.6rem;
  }
  .words {
    margin: 0;
    padding-left: 1.2rem;
  }
  .words li {
    margin: 0.2rem 0;
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
</style>
