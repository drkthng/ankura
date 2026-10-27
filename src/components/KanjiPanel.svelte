<script lang="ts">
  import { api, stateLabel, type KanjiCard, type NewStory } from "../lib/api";

  let { kanji, onresize }: { kanji: string; onresize: () => void } = $props();

  let card = $state<KanjiCard | null>(null);
  let error = $state("");
  let showStory = $state(false);
  let proposal = $state<NewStory | null>(null);
  let busy = $state<"" | "story" | "save" | "undo">("");
  let message = $state("");

  async function load() {
    error = "";
    try {
      card = await api.kanjiCard(kanji);
    } catch (e) {
      error = String(e);
    }
    requestAnimationFrame(onresize);
  }
  $effect(() => {
    kanji;
    card = null;
    proposal = null;
    message = "";
    showStory = false;
    load();
  });

  /** Stories are HTML in Anki; show them as text with line breaks. */
  const text = (html: string) => {
    const d = document.createElement("div");
    d.innerHTML = html.replace(/<br\s*\/?>/gi, "\n");
    return d.textContent ?? "";
  };

  async function run(kind: typeof busy, f: () => Promise<void>) {
    busy = kind;
    error = "";
    try {
      await f();
    } catch (e) {
      error = String(e);
    } finally {
      busy = "";
      requestAnimationFrame(onresize);
    }
  }
  const newStory = () =>
    run("story", async () => {
      proposal = await api.kanjiNewStory(kanji);
    });
  const save = () =>
    run("save", async () => {
      if (!proposal) return;
      const n = await api.kanjiSaveStory(kanji, proposal.story);
      proposal = null;
      message = `✓ Saved to ${n} ${n === 1 ? "note" : "notes"} in Anki.`;
      await load();
      showStory = true;
    });
  const undo = () =>
    run("undo", async () => {
      await api.kanjiUndoStory(kanji);
      message = "✓ The old story is back.";
      await load();
    });
</script>

<section class="kanji">
  {#if error}<p class="error small">{error}</p>{/if}
  {#if card}
    <header>
      <span class="big">{kanji}</span>
      {#if card.info}
        <span class="kw">{text(card.info.keyword)}</span>
        <span class="pill st-{card.info.state}">{stateLabel[card.info.state]}</span>
      {:else}
        <span class="muted small">Not in your kanji cards.</span>
      {/if}
    </header>

    {#if card.info}
      {#if proposal}
        <div class="compare">
          <p class="muted small">Your story</p>
          <p class="story old">{card.story_text || "(empty)"}</p>
          <p class="muted small">New story</p>
          <p class="story">{text(proposal.story)}</p>
          {#if proposal.primitives.length}
            <p class="muted small prims">{proposal.primitives.map((p) => `${p.component} ${p.name}`).join(" · ")}</p>
          {/if}
          <div class="row">
            <button class="primary" disabled={busy !== ""} onclick={save}>{busy === "save" ? "Saving…" : "Save to Anki"}</button>
            <button class="ghost" disabled={busy !== ""} onclick={newStory}>{busy === "story" ? "Writing…" : "Another"}</button>
            <button class="ghost" disabled={busy !== ""} onclick={() => (proposal = null)}>Keep mine</button>
          </div>
          <p class="muted small">Saving changes only the mnemonic field of this kanji's notes. Ankura keeps the old text, so you can undo.</p>
        </div>
      {:else}
        {#if showStory}
          <p class="story">{card.story_text || "(no story yet)"}</p>
        {:else}
          <button class="link" onclick={() => { showStory = true; requestAnimationFrame(onresize); }}>Show story</button>
        {/if}
        <div class="row">
          <button class="ghost" disabled={busy !== ""} onclick={newStory} title="The AI writes a new RTK-style story from the kanji's primitives">
            {busy === "story" ? "Writing a story…" : "New mnemonic"}
          </button>
          {#if card.last_edit}
            <button class="ghost" disabled={busy !== ""} onclick={undo} title="Put back the story Ankura replaced">
              {busy === "undo" ? "Undoing…" : "Undo Ankura's change"}
            </button>
          {/if}
        </div>
      {/if}
      {#if message}<p class="small ok">{message}</p>{/if}
    {/if}

    {#if card.words.length}
      <p class="muted small">Your words with {kanji}</p>
      <div class="words">
        {#each card.words as w}<span class="w st-{w.state}" title="{w.reading} · {stateLabel[w.state]}">{w.display}</span>{/each}
      </div>
    {/if}
  {:else if !error}
    <p class="muted small">Looking up {kanji}…</p>
  {/if}
</section>

<style>
  .kanji {
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
    display: grid;
    gap: 0.4rem;
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .big {
    font-family: var(--font-read);
    font-size: 2rem;
    line-height: 1;
  }
  .kw {
    font-weight: 600;
  }
  .pill {
    font-size: 0.75rem;
    padding: 0.1em 0.6em;
    border-radius: 999px;
    background: var(--surface-2);
  }
  .story {
    white-space: pre-line;
    margin: 0;
    font-size: 0.92rem;
  }
  .story.old {
    color: var(--muted);
  }
  .compare {
    display: grid;
    gap: 0.25rem;
  }
  .compare p.muted {
    margin: 0.3rem 0 0;
  }
  .prims {
    font-family: var(--font-read);
  }
  .row button {
    font-size: 0.85rem;
    padding: 0.2em 0.6em;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    justify-self: start;
    font-size: 0.88rem;
  }
  .words {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .w {
    font-family: var(--font-read);
    padding: 0.05em 0.45em;
    border-radius: 6px;
    background: var(--surface-2);
  }
  .w.st-learning {
    background: var(--st-learning-bg);
  }
  .w.st-new {
    background: var(--st-new-bg);
  }
  .small {
    font-size: 0.82rem;
    margin: 0;
  }
  .ok {
    color: var(--accent);
  }
</style>
