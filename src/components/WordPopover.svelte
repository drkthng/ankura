<script lang="ts">
  import { onMount } from "svelte";
  import { api, stateLabel, type ReaderToken, type WordInfo, type WordState } from "../lib/api";

  let {
    token,
    anchor,
    onclose,
    onmark,
    ondontknow,
    sentence = "",
    textId = null,
  }: {
    token: ReaderToken;
    anchor: DOMRect;
    onclose: () => void;
    onmark: (state: WordState | null) => void;
    ondontknow: () => void;
    /** The sentence the word appears in, for a card. */
    sentence?: string;
    textId?: string | null;
  } = $props();

  // --- Anki card ---
  let carding = $state<"" | "text" | "new">("");
  let cardMsg = $state("");
  async function makeCard(useSentence: boolean) {
    carding = useSentence ? "text" : "new";
    cardMsg = "";
    try {
      const d = await api.cardCreate(key, token.dictionary_form, useSentence ? sentence : null, textId);
      cardMsg = d.status === "added" ? "✓ Added to Anki" : "✓ In your card list (Cards)";
    } catch (e) {
      cardMsg = String(e);
    } finally {
      carding = "";
    }
  }

  let info = $state<WordInfo | null>(null);
  let error = $state("");
  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  onMount(() => {
    if (token.key) {
      api.wordInfo(token.key, token.dictionary_form).then((i) => (info = i), (e) => (error = String(e)));
    }
    place();
  });

  function place() {
    const w = 380;
    const left = Math.min(Math.max(12, anchor.left + anchor.width / 2 - w / 2), window.innerWidth - w - 12);
    const below = anchor.bottom + 10;
    const height = el?.offsetHeight ?? 260;
    const top = below + height > window.innerHeight - 12 ? Math.max(12, anchor.top - height - 10) : below;
    pos = { left, top };
  }

  $effect(() => {
    // Re-place once the content has loaded and the height is known.
    if (info) requestAnimationFrame(place);
  });

  const key = $derived(token.key!);
  const best = $derived(info?.entries[0]);
  const others = $derived(info?.entries.slice(1, 4) ?? []);
  const isMarked = $derived(token.source === "mark");
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} onresize={place} />

<div class="backdrop" role="presentation" onclick={onclose}></div>
<div class="pop" bind:this={el} style:left="{pos.left}px" style:top="{pos.top}px" role="dialog" aria-label="Word details">
  <header>
    <div class="word">
      <span class="lemma">{token.dictionary_form}</span>
      {#if key.reading && key.reading !== token.dictionary_form}<span class="reading">{key.reading}</span>{/if}
      {#if key.lemma !== token.dictionary_form}<span class="also">also written {key.lemma}</span>{/if}
    </div>
    <span class="state st-{token.state}">{token.source === "parts" ? "Known parts" : token.source === "basic" ? "Basic word" : stateLabel[token.state]}</span>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if info}
    {#if best}
      <ol class="senses">
        {#each best.senses.slice(0, 5) as s}<li>{s}</li>{/each}
      </ol>
    {:else}
      <p class="muted">No dictionary entry found.</p>
    {/if}
    {#if others.length}
      <details>
        <summary>Other entries ({others.length})</summary>
        {#each others as o}
          <p class="other"><strong>{o.expression}</strong> 【{o.reading}】 {o.senses.slice(0, 2).join(" / ")}</p>
        {/each}
      </details>
    {/if}
    {#if info.readings.length > 1}
      <p class="muted small">Readings in the dictionary: {info.readings.join("、")}</p>
    {/if}
    <p class="muted small">
      {#if info.anki}In Anki: {info.anki.cards} card{info.anki.cards === 1 ? "" : "s"}{#if info.anki.strength >= 1}, strength {Math.round(info.anki.strength)} days{/if} · {/if}
      Seen {info.stats.exposures}× · looked up {info.stats.lookups}×
    </p>
  {:else if !error}
    <p class="muted">Looking up…</p>
  {/if}

  <section class="card-row">
    <span class="muted small">Anki card:</span>
    {#if sentence}
      <button class="ghost" disabled={carding !== ""} onclick={() => makeCard(true)} title={sentence}>
        {carding === "text" ? "Adding…" : "with this sentence"}
      </button>
    {/if}
    <button class="ghost" disabled={carding !== ""} onclick={() => makeCard(false)} title="The AI writes a sentence in which only this word is new to you">
      {carding === "new" ? "Writing a sentence…" : "with a new sentence"}
    </button>
    {#if cardMsg}<span class="small card-msg" class:error={!cardMsg.startsWith("✓")}>{cardMsg}</span>{/if}
  </section>

  <footer>
    <button class="primary" onclick={ondontknow}>I don't know this</button>
    {#if token.state !== "marked_known"}
      <button onclick={() => onmark("marked_known")}>I know this</button>
    {/if}
    {#if token.state !== "ignored"}
      <button class="ghost" title="Names, foreign words: never highlight or count" onclick={() => onmark("ignored")}>Ignore</button>
    {/if}
    {#if isMarked}
      <button class="ghost" onclick={() => onmark(null)}>Clear mark</button>
    {/if}
  </footer>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
  }
  .pop {
    position: fixed;
    z-index: 11;
    width: 380px;
    max-height: 70vh;
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow);
    padding: 1rem 1.1rem 0.9rem;
    display: grid;
    gap: 0.6rem;
    animation: pop 0.12s ease-out;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.6rem;
  }
  .word {
    display: flex;
    flex-direction: column;
  }
  .lemma {
    font-family: var(--font-read);
    font-size: 1.7rem;
    line-height: 1.2;
  }
  .reading {
    color: var(--muted);
    font-size: 1rem;
  }
  .also {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .state {
    font-size: 0.78rem;
    padding: 0.15em 0.65em;
    border-radius: 999px;
    white-space: nowrap;
    background: var(--surface-2);
  }
  .st-unknown {
    background: var(--st-unknown-bg);
  }
  .st-new {
    background: var(--st-new-bg);
  }
  .st-learning {
    background: var(--st-learning-bg);
  }
  .st-known,
  .st-marked_known,
  .st-fading {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .senses {
    margin: 0;
    padding-left: 1.2rem;
    display: grid;
    gap: 0.2rem;
  }
  .other {
    margin: 0.3rem 0;
    font-size: 0.9rem;
  }
  details summary {
    cursor: pointer;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .small {
    font-size: 0.82rem;
    margin: 0;
  }
  .card-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem;
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
  }
  .card-row button {
    font-size: 0.85rem;
    padding: 0.2em 0.6em;
  }
  .card-msg {
    flex-basis: 100%;
    margin: 0;
  }
  footer {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    padding-top: 0.3rem;
    border-top: 1px solid var(--line);
  }
</style>
