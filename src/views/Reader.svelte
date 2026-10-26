<script lang="ts">
  import { onDestroy, onMount } from "svelte";
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

  let { id, onback }: { id: string; onback: () => void } = $props();

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
  let selected = $state<{ token: ReaderToken; rect: DOMRect } | null>(null);

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
    observer.disconnect();
    timers.forEach(clearTimeout);
    clearInterval(flushTimer);
    flush();
  });

  function open(token: ReaderToken, ev: MouseEvent) {
    if (!token.key) return;
    selected = { token, rect: (ev.currentTarget as HTMLElement).getBoundingClientRect() };
    pending.push({ kind: "lookup", key: token.key, text_id: id, display: token.dictionary_form });
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
    for (const p of doc.paragraphs) for (const t of p) if (t.key && keyId(t.key) === kid) Object.assign(t, { state, source: "mark" });
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

  const showRuby = (t: ReaderToken) =>
    t.has_kanji && furigana !== "none" && (furigana === "all" || !["known", "fading", "marked_known", "ignored"].includes(t.state));
  const isWord = (t: ReaderToken) => t.key !== null;
</script>

<div class="reader-page">
  <header class="toolbar">
    <button class="ghost" onclick={onback}>← Library</button>
    <h1 class="title">{meta?.title ?? ""}</h1>
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
      {#if extra}
        <p class="muted written">
          Written for you around {extra.targets.join("、")}{extra.topic ? ` · ${extra.topic}` : ""}
        </p>
      {/if}
    </div>
    <article class="text" class:hl={highlight} class:vertical>
      {#each doc.paragraphs as para, i}
        <p use:observe data-p={i}>
          {#each para as t}
            {#if isWord(t)}
              <span
                class="tok st-{t.state}"
                class:name={t.kind === "name"}
                class:selected={selected?.token === t}
                role="button"
                tabindex="0"
                onclick={(e) => open(t, e)}
                onkeydown={(e) => e.key === "Enter" && open(t, e as unknown as MouseEvent)}
              >
                {#each t.segments as s}
                  {#if s.ruby && showRuby(t)}<ruby>{s.text}<rt>{s.ruby}</rt></ruby>{:else}{s.text}{/if}
                {/each}
              </span>
            {:else}
              {#each t.segments as s}{s.text}{/each}
            {/if}
          {/each}
        </p>
      {/each}
    </article>
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
    margin: -4rem auto 4rem;
    padding: 0 2rem;
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
