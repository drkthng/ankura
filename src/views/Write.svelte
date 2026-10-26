<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AiSettings, type StudyWord } from "../lib/api";

  let { onopen, ongo }: { onopen: (id: string) => void; ongo: (view: "settings") => void } = $props();

  let ai = $state<AiSettings | null>(null);
  let words = $state<StudyWord[]>([]);
  let picked = $state<string[]>([]);
  let typed = $state("");
  let topic = $state("");
  let length = $state(500);
  let polite = $state(false);
  let busy = $state(false);
  let error = $state("");
  let started = $state(0);
  let elapsed = $state(0);

  onMount(async () => {
    try {
      [ai, words] = await Promise.all([api.aiSettings(), api.studyWords()]);
    } catch (e) {
      error = String(e);
    }
  });

  const MAX = 8;
  const groups = $derived([
    { reason: "looked_up", label: "You looked these up", hint: "words you clicked “I don't know this” on" },
    { reason: "fading", label: "Fading in Anki", hint: "known, but you're likely forgetting them" },
    { reason: "learning", label: "Still learning", hint: "" },
    { reason: "new", label: "New in Anki", hint: "cards you haven't studied yet" },
  ].map((g) => ({ ...g, words: words.filter((w) => w.reason === g.reason).slice(0, 16) })).filter((g) => g.words.length));

  function toggle(lemma: string) {
    if (picked.includes(lemma)) picked = picked.filter((p) => p !== lemma);
    else if (picked.length < MAX) picked = [...picked, lemma];
  }

  function addTyped() {
    for (const w of typed.split(/[、,\s]+/).map((x) => x.trim()).filter(Boolean)) {
      if (!picked.includes(w) && picked.length < MAX) picked = [...picked, w];
    }
    typed = "";
  }

  const IDEAS = ["a small mystery", "a day at the market", "cooking dinner", "a train journey", "a letter to a friend", "a ghost story", "news about a new invention"];

  async function write() {
    addTyped();
    if (!picked.length) return;
    busy = true;
    error = "";
    started = Date.now();
    const tick = setInterval(() => (elapsed = Math.round((Date.now() - started) / 1000)), 1000);
    try {
      const r = await api.generateText({ targets: picked, topic, length, polite });
      onopen(r.text.id);
    } catch (e) {
      error = String(e);
    } finally {
      clearInterval(tick);
      busy = false;
    }
  }
</script>

<div class="page">
  <h1>Write a text for me</h1>
  <p class="muted lead">
    Pick a few words to practise. Ankura asks the AI for a short text that uses them, written with words you already
    know, checks that at least 95% of it is familiar, and asks for a rewrite if not.
  </p>

  {#if ai && !ai.has_key}
    <section class="card notice">
      <p>Writing texts needs an API key from Anthropic (or OpenRouter).</p>
      <button class="primary" onclick={() => ongo("settings")}>Add a key in Settings</button>
    </section>
  {/if}

  <section class="card">
    <h2>1. Words to practise <span class="muted count">{picked.length}/{MAX}</span></h2>
    <div class="picked">
      {#each picked as p}
        <button class="chip on" onclick={() => toggle(p)} title="Remove">{p} ×</button>
      {/each}
      <input
        class="typed"
        placeholder={picked.length ? "add another…" : "type words, or pick below"}
        bind:value={typed}
        onkeydown={(e) => e.key === "Enter" && addTyped()}
        onblur={addTyped}
      />
    </div>
    {#each groups as g}
      <div class="group">
        <h3>{g.label} {#if g.hint}<span class="muted hint">· {g.hint}</span>{/if}</h3>
        <div class="chips">
          {#each g.words as w}
            <button
              class="chip"
              class:on={picked.includes(w.display)}
              onclick={() => toggle(w.display)}
              title={w.key.reading}
            >
              {w.display}{#if w.lookups > 1}<small> ×{w.lookups}</small>{/if}
            </button>
          {/each}
        </div>
      </div>
    {:else}
      <p class="muted">Sync Anki, or click “I don't know this” on words while reading, to get suggestions here.</p>
    {/each}
  </section>

  <section class="card">
    <h2>2. What about?</h2>
    <input class="topic" placeholder="Any topic, e.g. a detective story set in Osaka (optional)" bind:value={topic} />
    <div class="chips ideas">
      {#each IDEAS as idea}<button class="chip" onclick={() => (topic = idea)}>{idea}</button>{/each}
    </div>
    <div class="row opts">
      <label>Length
        <select bind:value={length}>
          <option value={300}>Short (~300 characters)</option>
          <option value={500}>Medium (~500)</option>
          <option value={900}>Long (~900)</option>
        </select>
      </label>
      <label>Style
        <select bind:value={polite}>
          <option value={false}>Plain (だ・である)</option>
          <option value={true}>Polite (です・ます)</option>
        </select>
      </label>
    </div>
  </section>

  <div class="row go">
    <button class="primary big" disabled={busy || (!picked.length && !typed.trim()) || !ai?.has_key} onclick={write}>
      {busy ? "Writing…" : "Write it"}
    </button>
    {#if busy}
      <span class="muted">The AI is writing and Ankura is checking the result ({elapsed}s). This usually takes under a minute.</span>
    {/if}
  </div>
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .page {
    max-width: 880px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
  .lead {
    margin: -0.6rem 0 0;
  }
  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    border-color: var(--gold);
  }
  .notice p {
    margin: 0;
  }
  .count {
    font-weight: 400;
    font-size: 0.85rem;
    margin-left: 0.4rem;
  }
  .picked {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
    padding: 0.5rem;
    border: 1px solid var(--line);
    border-radius: 8px;
    margin-bottom: 0.8rem;
    min-height: 2.8rem;
  }
  .typed {
    border: none;
    flex: 1;
    min-width: 12rem;
    padding: 0.3rem;
  }
  .typed:focus {
    outline: none;
  }
  .group h3 {
    font-size: 0.9rem;
    margin: 0.6rem 0 0.4rem;
  }
  .hint {
    font-weight: 400;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }
  .chip {
    font-family: var(--font-read);
    padding: 0.25em 0.75em;
    border-radius: 999px;
    font-size: 1rem;
  }
  .chip small {
    color: var(--muted);
    font-family: var(--font-ui);
  }
  .chip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .ideas .chip {
    font-family: var(--font-ui);
    font-size: 0.85rem;
  }
  .topic {
    width: 100%;
    margin-bottom: 0.6rem;
  }
  .opts {
    margin-top: 0.8rem;
  }
  .opts label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .big {
    font-size: 1.05rem;
    padding: 0.6em 1.6em;
  }
</style>
