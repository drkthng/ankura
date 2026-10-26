<script lang="ts">
  import { onMount } from "svelte";
  import { api, type CardDraft, type CardOptions, type StudyWord } from "../lib/api";

  let { ongo }: { ongo: (view: "settings") => void } = $props();

  let drafts = $state<CardDraft[]>([]);
  let options = $state<CardOptions | null>(null);
  let looked = $state<StudyWord[]>([]);
  let error = $state("");
  /** Per-card state: what's running and the last error. */
  let busy = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let chosen = $state<Record<string, boolean>>({});
  let adding = $state(false);
  let making = $state<Record<string, boolean>>({});
  /** The note type's fields in Anki's order (stored drafts come back sorted by name). */
  let fieldOrder = $state<string[]>([]);
  const ordered = (fields: Record<string, string>) =>
    Object.entries(fields).sort(([a], [b]) => {
      const i = fieldOrder.indexOf(a), j = fieldOrder.indexOf(b);
      return (i < 0 ? 999 : i) - (j < 0 ? 999 : j);
    });

  const open = $derived(drafts.filter((d) => d.status === "draft"));
  const added = $derived(drafts.filter((d) => d.status === "added"));
  const ready = $derived(open.filter((d) => d.fields && chosen[d.id] !== false));
  const inList = $derived(new Set(drafts.map((d) => d.key.lemma)));

  async function load() {
    drafts = await api.cardDrafts();
  }

  onMount(async () => {
    try {
      await load();
      [options, looked] = await Promise.all([
        api.cardOptions().catch((e) => {
          error = String(e);
          return null;
        }),
        api.studyWords().then((w) => w.filter((x) => x.reason === "looked_up")),
      ]);
      if (options?.note_type && options?.deck) fieldOrder = await api.cardFieldNames().catch(() => []);
      fillAll();
    } catch (e) {
      error = String(e);
    }
  });

  function replace(d: CardDraft) {
    drafts = drafts.map((x) => (x.id === d.id ? d : x));
  }

  async function run(id: string, what: string, f: () => Promise<void>) {
    busy = { ...busy, [id]: what };
    errors = { ...errors, [id]: "" };
    try {
      await f();
    } catch (e) {
      errors = { ...errors, [id]: String(e) };
    } finally {
      const { [id]: _, ...rest } = busy;
      busy = rest;
    }
  }

  /** Fills the cards that have no fields yet, one at a time. */
  let filling = false;
  async function fillAll() {
    if (filling || !options?.note_type || !options?.deck) return;
    filling = true;
    try {
      for (const d of drafts.filter((x) => x.status === "draft" && !x.fields && !errors[x.id])) {
        await run(d.id, "Filling in…", async () => replace(await api.cardFill(d.id)));
      }
    } finally {
      filling = false;
    }
  }

  const fill = (d: CardDraft) => run(d.id, "Filling in…", async () => replace(await api.cardFill(d.id)));
  const newSentence = (d: CardDraft) =>
    run(d.id, "Writing a new sentence…", async () => {
      replace(await api.cardNewSentence(d.id));
      await fill(drafts.find((x) => x.id === d.id)!);
    });
  const byHand = (d: CardDraft) =>
    run(d.id, "", async () => {
      const names = await api.cardFieldNames();
      const fields = Object.fromEntries(names.map((n) => [n, n === names[0] ? d.sentence : ""]));
      await api.cardSetFields(d.id, fields);
      replace({ ...d, fields });
    });
  async function discard(d: CardDraft) {
    await api.cardDiscard(d.id);
    drafts = drafts.filter((x) => x.id !== d.id);
  }
  async function saveField(d: CardDraft, name: string, value: string) {
    if (!d.fields || d.fields[name] === value) return;
    const fields = { ...d.fields, [name]: value };
    replace({ ...d, fields });
    await api.cardSetFields(d.id, fields).catch((e) => (errors = { ...errors, [d.id]: String(e) }));
  }

  async function addChosen() {
    adding = true;
    for (const d of ready) {
      await run(d.id, "Adding…", async () => replace(await api.cardAdd(d.id)));
    }
    adding = false;
  }

  async function cardFor(w: StudyWord) {
    making = { ...making, [w.key.lemma]: true };
    try {
      const d = await api.cardCreate(w.key, w.display, null, null);
      drafts = [...drafts, d];
      fillAll();
    } catch (e) {
      error = String(e);
    } finally {
      making = { ...making, [w.key.lemma]: false };
    }
  }

  const targetSet = $derived(!!options?.note_type && !!options?.deck);
</script>

<div class="page">
  <h1>Cards</h1>
  <p class="muted lead">
    Cards you make while reading wait here. Ankura fills them in like your own notes; check them, change what you
    like, and add the ones you want to Anki. Ankura only ever adds notes (tagged <code>ankura</code>), never changes
    your existing ones.
  </p>

  {#if error}<p class="error">{error}</p>{/if}

  {#if options && !targetSet}
    <section class="card notice">
      <p>Choose which note type and deck new cards go into.</p>
      <button class="primary" onclick={() => ongo("settings")}>Choose in Settings</button>
    </section>
  {:else if options}
    <p class="muted small">New cards go into <strong>{options.note_type}</strong> → deck <strong>{options.deck}</strong>{options.auto_add ? " (added without the list)" : ""}.</p>
  {/if}

  {#if open.length}
    <div class="row top">
      <button class="primary" disabled={adding || !ready.length || !targetSet} onclick={addChosen}>
        {adding ? "Adding…" : `Add ${ready.length} to Anki`}
      </button>
      <span class="muted small">Untick a card to keep it here for now.</span>
    </div>
  {/if}

  {#each open as d (d.id)}
    <section class="card draft">
      <header>
        <label class="pick">
          <input type="checkbox" checked={chosen[d.id] !== false} disabled={!d.fields} onchange={(e) => (chosen = { ...chosen, [d.id]: e.currentTarget.checked })} />
          <span class="word">{d.display}</span>
          <span class="muted">{d.key.reading}</span>
        </label>
        <span class="src muted small">{d.sentence_source === "new" ? "new sentence" : "from your reading"}</span>
      </header>
      <p class="sentence">{d.sentence}</p>
      {#if busy[d.id]}
        <p class="muted small">{busy[d.id]}</p>
      {:else if d.fields}
        <div class="fields">
          {#each ordered(d.fields) as [name, value] (name)}
            <label for="{d.id}-{name}">{name}</label>
            <textarea id="{d.id}-{name}" rows={value.length > 60 || value.includes("<br") ? 3 : 1} value={value} onchange={(e) => saveField(d, name, e.currentTarget.value)}></textarea>
          {/each}
        </div>
      {/if}
      {#if errors[d.id]}
        <p class="error small">{errors[d.id]}</p>
      {/if}
      <div class="row actions">
        {#if !d.fields && !busy[d.id]}
          <button onclick={() => fill(d)} disabled={!targetSet}>Fill in with AI</button>
          <button class="ghost" onclick={() => byHand(d)} disabled={!targetSet}>Fill in by hand</button>
        {/if}
        <button class="ghost" disabled={!!busy[d.id]} onclick={() => newSentence(d)}>New sentence</button>
        <button class="ghost danger" disabled={!!busy[d.id]} onclick={() => discard(d)}>Discard</button>
      </div>
    </section>
  {:else}
    <section class="card empty">
      <p class="muted">
        No cards waiting. In the reader, click a word (or select a phrase) and choose “Anki card”. Texts written for
        you have a “Make Anki cards” button under the example sentences.
      </p>
    </section>
  {/each}

  {#if looked.some((w) => !inList.has(w.key.lemma))}
    <section class="card">
      <h2>Words you looked up</h2>
      <p class="muted small">Words you marked “I don't know this”. Each card gets a new sentence in which only that word is new to you.</p>
      <div class="chips">
        {#each looked.filter((w) => !inList.has(w.key.lemma)) as w (w.key.lemma)}
          <button class="chip" disabled={making[w.key.lemma]} onclick={() => cardFor(w)} title={w.key.reading}>
            {making[w.key.lemma] ? "…" : "+"} {w.display}
          </button>
        {/each}
      </div>
    </section>
  {/if}

  {#if added.length}
    <section class="card">
      <h2>Added to Anki</h2>
      <ul class="added">
        {#each added as d (d.id)}<li><span class="word">{d.display}</span> <span class="muted">{d.sentence}</span></li>{/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .page {
    max-width: 880px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .lead {
    margin: -0.4rem 0 0;
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
  .small {
    font-size: 0.85rem;
  }
  .draft header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }
  .pick {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }
  .word {
    font-family: var(--font-read);
    font-size: 1.3rem;
  }
  .sentence {
    font-family: var(--font-read);
    font-size: 1.15rem;
    margin: 0.5rem 0;
  }
  .fields {
    display: grid;
    grid-template-columns: 10rem 1fr;
    gap: 0.4rem 0.8rem;
    align-items: start;
    margin: 0.6rem 0;
  }
  .fields label {
    color: var(--muted);
    font-size: 0.85rem;
    padding-top: 0.35rem;
  }
  .fields textarea {
    width: 100%;
    font-family: var(--font-read);
    resize: vertical;
  }
  .actions {
    margin-top: 0.4rem;
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
  }
  .added {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.3rem;
  }
  .added .word {
    font-size: 1rem;
    margin-right: 0.5rem;
  }
</style>
