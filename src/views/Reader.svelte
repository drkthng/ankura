<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import {
    api,
    keyId,
    type Coverage,
    type NewEvent,
    type ReaderDoc,
    type ReaderToken,
    type TextExtra,
    type TextMeta,
    type WordState,
  } from "../lib/api";
  import CoverageBar from "../components/CoverageBar.svelte";
  import WordPopover from "../components/WordPopover.svelte";
  import * as speech from "../lib/speech";
  import Quiz from "../components/Quiz.svelte";

  let {
    id,
    onback,
    oncards,
    today = null,
  }: {
    id: string;
    onback: () => void;
    oncards: () => void;
    /** Opened from Today (ADR 0019): the session's steps, and how to report one done. */
    today?: { done: string[]; onstep: (step: string) => void } | null;
  } = $props();
  let showQuiz = $state(false);
  const step = (s: string) => today?.onstep(s);
  const stepDone = (s: string) => !!today?.done.includes(s);

  type Furigana = "unknown" | "all" | "none";
  const pref = <T extends string>(k: string, d: T): T => {
    try {
      return (localStorage.getItem(k) as T) ?? d;
    } catch {
      return d;
    }
  };
  let furigana = $state<Furigana>(pref("ankura.furigana", "unknown"));
  let highlight = $state(pref<string>("ankura.highlight", "on") === "on");
  let vertical = $state(pref<string>("ankura.vertical", "off") === "on");
  $effect(() => {
    try {
      localStorage.setItem("ankura.furigana", furigana);
      localStorage.setItem("ankura.highlight", highlight ? "on" : "off");
      localStorage.setItem("ankura.vertical", vertical ? "on" : "off");
    } catch {}
  });

  let meta = $state<TextMeta | null>(null);
  let extra = $state<TextExtra | null>(null);
  let doc = $state<ReaderDoc | null>(null);
  let error = $state("");
  /** `at`: index of the sentence in `sentences`, to listen from there. */
  let selected = $state<{ token: ReaderToken; rect: DOMRect; sentence: string; at: number } | null>(null);

  onMount(async () => {
    try {
      const r = await api.openText(id);
      meta = r.meta;
      doc = r.doc;
      extra = r.extra;
    } catch (e) {
      error = String(e);
    }
  });

  // --- telemetry: exposures (paragraph visible ≥ 3 s, once per paragraph per visit) and lookups ---
  let pending: NewEvent[] = [];
  const flush = () => {
    if (pending.length) {
      const batch = pending;
      pending = [];
      api.recordEvents(batch).catch(() => (pending = batch.concat(pending)));
    }
  };
  const flushTimer = setInterval(flush, 5000);
  const exposed = new Set<number>();
  const timers = new Map<number, ReturnType<typeof setTimeout>>();
  const observer = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        const i = Number((e.target as HTMLElement).dataset.p);
        if (e.isIntersecting && !exposed.has(i)) {
          timers.set(
            i,
            setTimeout(() => {
              exposed.add(i);
              const chars = (doc?.paragraphs[i] ?? []).reduce((n, t) => n + t.surface.replace(/\s/g, "").length, 0);
              if (chars) api.recordReading(chars).catch(() => {});
              for (const t of doc?.paragraphs[i] ?? []) {
                if (t.key && t.kind === "word") pending.push({ kind: "exposure", key: t.key, text_id: id, display: t.dictionary_form });
              }
            }, 3000),
          );
        } else if (!e.isIntersecting) {
          clearTimeout(timers.get(i));
          timers.delete(i);
        }
      }
    },
    { threshold: 0.6 },
  );
  function observe(node: HTMLElement) {
    observer.observe(node);
    return { destroy: () => observer.unobserve(node) };
  }
  onDestroy(() => {
    stopPlay();
    observer.disconnect();
    timers.forEach(clearTimeout);
    clearInterval(flushTimer);
    flush();
  });

  const ENDS = /[。！？!?]$/;

  /** The sentence of a paragraph that token `i` belongs to. */
  function sentenceAround(para: ReaderToken[], i: number): string {
    let a = i;
    while (a > 0 && !ENDS.test(para[a - 1].surface)) a--;
    let b = i;
    while (b < para.length - 1 && !ENDS.test(para[b].surface)) b++;
    while (b < para.length - 1 && /^[」』）)]/.test(para[b + 1].surface)) b++;
    return para.slice(a, b + 1).map((t) => t.surface).join("").trim();
  }

  function open(token: ReaderToken, ev: MouseEvent, para: ReaderToken[]) {
    if (!token.key || !doc) return;
    const j = para.indexOf(token);
    const sentence = sentenceAround(para, j);
    const at = sentIndex[doc.paragraphs.indexOf(para)]?.[j] ?? 0;
    selected = { token, rect: (ev.currentTarget as HTMLElement).getBoundingClientRect(), sentence, at };
    pending.push({ kind: "lookup", key: token.key, text_id: id, display: token.dictionary_form });
  }

  /** Selecting several words looks them up as one phrase (気になります → 気になる). */
  let article: HTMLElement | undefined = $state();
  async function selectPhrase() {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || !sel.rangeCount || !doc || selected) return;
    const range = sel.getRangeAt(0);
    if (!article?.contains(range.commonAncestorContainer)) return;
    const frag = range.cloneContents();
    frag.querySelectorAll("rt").forEach((r) => r.remove()); // furigana isn't part of the text
    const text = (frag.textContent ?? "").replace(/\s+/g, "");
    if (text.length < 2) return;
    const rect = range.getBoundingClientRect();
    const p = (range.startContainer.nodeType === 1 ? (range.startContainer as Element) : range.startContainer.parentElement)?.closest("p[data-p]") as HTMLElement | null;
    const para = p ? doc.paragraphs[Number(p.dataset.p)] : [];
    const paraText = para.map((t) => t.surface).join("");
    const sentence =
      paraText.split(/(?<=[。！？!?][」』）)]?)/).find((x) => x.includes(text))?.trim() ?? paraText.trim();
    try {
      const token = await api.phraseInfo(text);
      if (!token?.key) return;
      sel.removeAllRanges();
      const pi = p ? Number(p.dataset.p) : -1;
      const at = Math.max(0, sentences.findIndex((x) => x.p === pi && x.text.includes(text)));
      selected = { token, rect, sentence, at };
      pending.push({ kind: "lookup", key: token.key, text_id: id, display: token.dictionary_form });
    } catch (e) {
      error = String(e);
    }
  }

  // --- cards for a written text's target words ---
  let cardsMsg = $state("");
  async function cardsForTargets() {
    cardsMsg = "";
    try {
      const made = await api.cardsForText(id);
      cardsMsg = made.length ? `${made.length} card${made.length === 1 ? "" : "s"} added to your card list.` : "These words are already in your card list.";
    } catch (e) {
      cardsMsg = String(e);
    }
  }

  function coverageOf(d: ReaderDoc): Coverage {
    const c: Coverage = { words: 0, known: 0, learning: 0, new: 0, unknown: 0, ratio: 0 };
    for (const p of d.paragraphs)
      for (const t of p) {
        if (t.kind !== "word") continue;
        c.words++;
        if (["known", "fading", "marked_known", "ignored"].includes(t.state)) c.known++;
        else if (t.state === "learning") c.learning++;
        else if (t.state === "new") c.new++;
        else c.unknown++;
      }
    c.ratio = c.words ? c.known / c.words : 0;
    return c;
  }

  /** Applies a mark to every occurrence of the word. Clearing a mark reloads, to get Anki's state back. */
  async function mark(state: WordState | null) {
    if (!selected?.token.key || !doc) return;
    const k = selected.token.key;
    await api.setWordMark(k, state);
    selected = null;
    if (state === null) {
      doc = (await api.openText(id)).doc;
      return;
    }
    const kid = keyId(k);
    let found = false;
    for (const p of doc.paragraphs)
      for (const t of p)
        if (t.key && keyId(t.key) === kid) {
          Object.assign(t, { state, source: "mark" });
          found = true;
        }
    // A selected phrase isn't a token yet: reload, so it shows as one word.
    if (!found) {
      doc = (await api.openText(id)).doc;
      return;
    }
    doc.coverage = coverageOf(doc);
  }

  async function dontKnow() {
    if (!selected?.token.key) return;
    pending.push({ kind: "dont_know", key: selected.token.key, text_id: id, display: selected.token.dictionary_form });
    const wasMarked = selected.token.state === "marked_known";
    if (wasMarked) await mark(null);
    selected = null;
    flush();
  }

  // --- listening (ADR 0017): sentence by sentence, the current one highlighted ---
  type Sentence = { p: number; a: number; b: number; text: string };
  /** Sentences in reading order; token [p][j] belongs to sentIndex[p][j]. */
  const split = $derived.by(() => {
    const out: Sentence[] = [];
    const index: number[][] = [];
    (doc?.paragraphs ?? []).forEach((para, p) => {
      index.push([]);
      let a = 0;
      for (let j = 0; j < para.length; j++) {
        index[p].push(out.length);
        const end = j === para.length - 1 || (ENDS.test(para[j].surface) && !/^[」』）)]/.test(para[j + 1].surface));
        if (end) {
          const text = para.slice(a, j + 1).map((t) => t.surface).join("").trim();
          if (text) out.push({ p, a, b: j, text });
          else index[p].splice(a);
          a = j + 1;
        }
      }
    });
    return { out, index };
  });
  const sentences = $derived(split.out);
  const sentIndex = $derived(split.index);
  let playState = $state<"idle" | "playing" | "paused">("idle");
  let cur = $state(-1);
  let voice = $state("");
  let playError = $state("");
  let rate = $state(1);
  let run = 0;
  onMount(() => {
    speech.load().then((s) => (rate = s.speed)).catch(() => {});
  });

  async function play(from = Math.max(0, cur)) {
    const me = ++run;
    playError = "";
    playState = "playing";
    voice = (await speech.current().catch(() => null))?.label ?? "";
    for (let i = from; i < sentences.length; i++) {
      if (run !== me) return;
      cur = i;
      await tick();
      document.querySelector(".text .speaking")?.scrollIntoView({ block: "center", behavior: "smooth" });
      try {
        await speech.say(sentences[i].text, "narrator", id);
      } catch (e) {
        if (run === me) {
          playError = String(e).replace(/^Error: /, "");
          playState = "idle";
        }
        return;
      }
    }
    if (run === me) {
      playState = "idle";
      cur = -1;
      if (from === 0) step("listened");
    }
  }
  function pausePlay() {
    speech.pause();
    playState = "paused";
  }
  function resumePlay() {
    speech.resume();
    playState = "playing";
  }
  function stopPlay() {
    run++;
    speech.stop();
    playState = "idle";
    cur = -1;
  }
  function togglePlay() {
    if (playState === "playing") pausePlay();
    else if (playState === "paused") resumePlay();
    else play();
  }
  function onkey(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (e.key.toLowerCase() === "p" && !e.ctrlKey && !e.metaKey && tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT") togglePlay();
  }

  const showRuby = (t: ReaderToken) =>
    t.has_kanji && furigana !== "none" && (furigana === "all" || !["known", "fading", "marked_known", "ignored"].includes(t.state));
  const isWord = (t: ReaderToken) => t.key !== null;
</script>

<svelte:window onmouseup={selectPhrase} onkeydown={onkey} />

<div class="reader-page">
  <header class="toolbar">
    <button class="ghost" onclick={onback}>← {today ? "Today" : "Library"}</button>
    <h1 class="title">{meta?.title ?? ""}</h1>
    <div class="listen">
      {#if playState === "playing"}
        <button onclick={pausePlay} title="Pause (P)">⏸ Pause</button>
      {:else}
        <button onclick={togglePlay} title="Read the text aloud (P)">▶ {playState === "paused" ? "Resume" : "Listen"}</button>
      {/if}
      {#if playState !== "idle"}<button class="ghost" onclick={stopPlay} title="Stop">⏹</button>{/if}
      <select bind:value={rate} onchange={() => speech.setSpeed(rate)} title="Speed">
        {#each [0.7, 0.85, 1, 1.15, 1.3] as r}<option value={r}>{r}×</option>{/each}
      </select>
    </div>
    <div class="controls">
      <label>
        Furigana
        <select bind:value={furigana}>
          <option value="unknown">Words I don't know</option>
          <option value="all">All words</option>
          <option value="none">Off</option>
        </select>
      </label>
      <label><input type="checkbox" bind:checked={highlight} /> Colours</label>
      <label><input type="checkbox" bind:checked={vertical} /> Vertical</label>
    </div>
  </header>

  {#if error}<p class="error pad">{error}</p>{/if}

  {#if doc}
    <div class="summary">
      <CoverageBar coverage={doc.coverage} />
      <p class="muted tip">Tip: drag across several words to look them up as one phrase. <kbd>P</kbd> reads the text aloud.</p>
      {#if playState !== "idle" && voice}<p class="muted tip">Voice: {voice}</p>{/if}
      {#if extra?.episode}<p class="muted written">Episode {extra.episode} of a serial story</p>{/if}
      {#if today}
        <div class="session">
          <strong>Today</strong>
          <button class:done={stepDone("read")} onclick={() => step("read")}>{stepDone("read") ? "✓ Read" : "1 · Done reading"}</button>
          <button class:done={stepDone("listened")} onclick={() => play(0)} title="Read aloud from the start; done when it reaches the end">
            {stepDone("listened") ? "✓ Listened" : "2 · Listen (P)"}
          </button>
          <button class:done={stepDone("questions")} onclick={() => (showQuiz = true)}>{stepDone("questions") ? "✓ Questions" : "3 · Questions"}</button>
          <button class="ghost" onclick={onback}>Back to Today</button>
        </div>
      {/if}
      {#if playError}<p class="error tip">{playError}</p>{/if}
      {#if extra}
        <p class="muted written">
          Written for you around {extra.targets.join("、")}{extra.topic ? ` · ${extra.topic}` : ""}
        </p>
      {/if}
    </div>
    <article class="text" class:hl={highlight} class:vertical bind:this={article}>
      {#each doc.paragraphs as para, i}
        <p use:observe data-p={i}>
          {#each para as t, j}
            {#if isWord(t)}
              <span
                class="tok st-{t.state}"
                class:speaking={cur >= 0 && sentIndex[i]?.[j] === cur}
                class:name={t.kind === "name"}
                class:selected={selected?.token === t}
                role="button"
                tabindex="0"
                onclick={(e) => window.getSelection()?.isCollapsed !== false && open(t, e, para)}
                onkeydown={(e) => e.key === "Enter" && open(t, e as unknown as MouseEvent, para)}
              >
                {#each t.segments as s}
                  {#if s.ruby && showRuby(t)}<ruby>{s.text}<rt>{s.ruby}</rt></ruby>{:else}{s.text}{/if}
                {/each}
              </span>
            {:else}
              <span class:speaking={cur >= 0 && sentIndex[i]?.[j] === cur}>{#each t.segments as s}{s.text}{/each}</span>
            {/if}
          {/each}
        </p>
      {/each}
    </article>
    <section class="check">
      {#if showQuiz}
        <Quiz textId={id} ondone={() => step("questions")} />
      {:else}
        <button onclick={() => (showQuiz = true)}>Check yourself: questions about this text</button>
      {/if}
    </section>
    {#if extra?.examples?.length}
      <section class="examples">
        <h2>Example sentences</h2>
        <ul>
          {#each extra.examples as ex}
            <li>
              <span class="ex-word">{ex.word}</span>
              <span class="ex-sentence">{ex.sentence}</span>
              <span class="muted ex-tr">{ex.translation}</span>
            </li>
          {/each}
        </ul>
        <div class="row cards-row">
          <button onclick={cardsForTargets}>Make Anki cards for {extra.targets.join("、")}</button>
          {#if cardsMsg}
            <span class="muted">{cardsMsg}</span>
            {#if cardsMsg.includes("card list")}<button class="ghost" onclick={oncards}>Open Cards</button>{/if}
          {/if}
        </div>
      </section>
    {/if}
  {:else if !error}
    <p class="muted pad">Reading the text…</p>
  {/if}
</div>

{#if selected}
  <WordPopover
    token={selected.token}
    anchor={selected.rect}
    onclose={() => (selected = null)}
    onmark={mark}
    ondontknow={dontKnow}
    sentence={selected.sentence}
    textId={id}
    onlisten={() => {
      const at = selected?.at ?? 0;
      selected = null;
      play(at);
    }}
    onreading={async () => {
      selected = null;
      doc = (await api.openText(id)).doc;
    }}
  />
{/if}

<style>
  .reader-page {
    min-height: 100%;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.7rem 1.5rem;
    background: color-mix(in srgb, var(--bg) 92%, transparent);
    backdrop-filter: blur(6px);
    border-bottom: 1px solid var(--line);
  }
  .title {
    flex: 1;
    margin: 0;
    font-size: 1.05rem;
    font-family: var(--font-read);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .session {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
    margin-top: 0.8rem;
    padding: 0.6rem 0.8rem;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--surface);
  }
  .session button.done {
    color: var(--accent);
    border-color: var(--accent);
  }
  .check {
    max-width: 44em;
    width: 100%;
    margin: -4rem auto 5rem;
    padding: 0 2rem;
  }
  .text.vertical + .check {
    margin-top: 1.5rem;
  }
  .listen {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }
  .listen select {
    font-size: 0.85rem;
    padding: 0.3em 0.4em;
  }
  .speaking {
    background: color-mix(in srgb, var(--gold) 22%, transparent);
    border-radius: 3px;
  }
  .hl .tok.speaking {
    box-shadow: inset 0 -2px 0 var(--gold);
  }
  kbd {
    font-family: var(--font-ui);
    font-size: 0.75rem;
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 0.35em;
  }
  .controls {
    display: flex;
    gap: 1rem;
    align-items: center;
    font-size: 0.88rem;
    color: var(--muted);
  }
  .controls label {
    display: flex;
    gap: 0.35rem;
    align-items: center;
  }
  .summary {
    max-width: 44em;
    width: 100%;
    margin: 1.6rem auto 0;
    padding: 0 2rem;
  }
  .pad {
    padding: 2rem;
  }
  .written {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
  }
  .examples {
    max-width: 44em;
    width: 100%;
    margin: 0 auto 4rem;
    padding: 0 2rem;
  }
  /* Vertical text has no bottom padding to pull back into. */
  .text.vertical + .examples {
    margin-top: 1.5rem;
  }
  .tip {
    margin: 0.4rem 0 0;
    font-size: 0.8rem;
  }
  .cards-row {
    margin-top: 1rem;
  }
  .examples h2 {
    font-size: 1rem;
    color: var(--muted);
  }
  .examples ul {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.7rem;
  }
  .examples li {
    display: grid;
    grid-template-columns: 6rem 1fr;
    gap: 0.1rem 1rem;
  }
  .ex-word {
    font-family: var(--font-read);
    font-weight: 600;
    grid-row: span 2;
  }
  .ex-sentence {
    font-family: var(--font-read);
    font-size: 1.1rem;
  }
  .ex-tr {
    font-size: 0.88rem;
  }
  .text {
    max-width: 44em;
    width: 100%;
    margin: 0 auto;
    padding: 1.4rem 2rem 6rem;
    font-family: var(--font-read);
    font-size: 1.45rem;
    line-height: 2.25;
    letter-spacing: 0.02em;
  }
  .text p {
    margin: 0 0 1.1em;
  }
  .text.vertical {
    writing-mode: vertical-rl;
    max-width: none;
    height: calc(100vh - 12rem);
    overflow-x: auto;
    padding: 1.4rem 2rem;
  }
  .text.vertical p {
    margin: 0 0 0 1.1em;
  }
  rt {
    font-size: 0.5em;
    color: var(--muted);
    letter-spacing: 0;
  }
  .tok {
    cursor: pointer;
    border-radius: 3px;
    transition: background 0.1s;
    padding: 0.04em 0;
  }
  .tok:hover,
  .tok.selected {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .hl .st-unknown {
    background: var(--st-unknown-bg);
    box-shadow: inset 0 -2px 0 var(--st-unknown-line);
  }
  .hl .st-new {
    background: var(--st-new-bg);
    box-shadow: inset 0 -2px 0 var(--st-new-line);
  }
  .hl .st-learning {
    background: var(--st-learning-bg);
    box-shadow: inset 0 -2px 0 var(--st-learning-line);
  }
  .hl .st-fading {
    text-decoration: underline dotted var(--st-fading-line);
    text-underline-offset: 0.3em;
  }
  .hl .tok.name.st-unknown {
    background: transparent;
    box-shadow: inset 0 -1px 0 var(--line);
  }
  .vertical .tok {
    padding: 0;
  }
  .hl.vertical .st-unknown,
  .hl.vertical .st-new,
  .hl.vertical .st-learning {
    box-shadow: none;
  }
</style>
