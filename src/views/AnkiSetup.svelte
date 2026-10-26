<script lang="ts">
  import { onMount } from "svelte";
  import { api, type MappingPreview, type NoteType, type Status, type SyncReport } from "../lib/api";

  let { status, onchange }: { status: Status | null; onchange: () => void } = $props();

  let url = $state("");
  let query = $state("");
  let version = $state<number | null>(null);
  let connectError = $state("");
  let connecting = $state(false);
  let noteTypes = $state<NoteType[]>([]);
  let loadingTypes = $state(false);

  // Per note type: the fields being edited, and the latest preview.
  type Draft = { target: string; reading: string; preview: MappingPreview | null; busy: boolean; error: string };
  let drafts = $state<Record<number, Draft>>({});

  let syncing = $state(false);
  let report = $state<SyncReport | null>(null);
  let syncError = $state("");

  $effect(() => {
    if (status && !url) {
      url = status.anki_url;
      query = status.anki_query;
    }
  });

  const WORD_FIELDS = ["expression", "vocab", "vocabkanji", "word", "単語", "語彙", "kanji", "front", "表面", "term", "japanese"];
  const READING_FIELDS = ["reading", "vocabkana", "kana", "読み", "よみ", "furigana", "hiragana"];
  function guess(fields: string[], names: string[]): string {
    for (const n of names) {
      const f = fields.find((x) => x.toLowerCase().replace(/[\s_-]/g, "") === n);
      if (f) return f;
    }
    for (const n of names) {
      const f = fields.find((x) => x.toLowerCase().includes(n));
      if (f) return f;
    }
    return "";
  }

  async function connect() {
    connecting = true;
    connectError = "";
    try {
      await api.setSetting("anki_url", url.trim());
      version = await api.ankiCheck();
      await loadTypes();
    } catch (e) {
      version = null;
      connectError = String(e);
    } finally {
      connecting = false;
    }
  }

  async function loadTypes() {
    loadingTypes = true;
    try {
      noteTypes = await api.ankiNoteTypes();
      const d: Record<number, Draft> = {};
      for (const nt of noteTypes) {
        d[nt.id] = {
          target: nt.mapping?.target_field ?? "",
          reading: nt.mapping?.reading_field ?? "",
          preview: null,
          busy: false,
          error: "",
        };
      }
      drafts = d;
    } finally {
      loadingTypes = false;
    }
  }

  onMount(() => {
    if (status) connect();
  });

  const mappingOf = (nt: NoteType) => ({
    model_id: nt.id,
    model_name: nt.name,
    target_field: drafts[nt.id].target,
    reading_field: drafts[nt.id].reading || null,
  });

  async function preview(nt: NoteType) {
    const d = drafts[nt.id];
    d.busy = true;
    d.error = "";
    try {
      d.preview = await api.ankiPreview(mappingOf(nt));
    } catch (e) {
      d.error = String(e);
    } finally {
      d.busy = false;
    }
  }

  let saving = $state(0);
  async function save(nt: NoteType) {
    saving++;
    try {
      await api.saveMapping(mappingOf(nt));
      nt.mapping = mappingOf(nt);
      onchange();
    } finally {
      saving--;
    }
  }

  async function unmap(nt: NoteType) {
    await api.deleteMapping(nt.id);
    nt.mapping = null;
    drafts[nt.id].target = "";
    drafts[nt.id].reading = "";
    drafts[nt.id].preview = null;
    onchange();
  }

  function suggest(nt: NoteType) {
    drafts[nt.id].target = guess(nt.fields, WORD_FIELDS);
    drafts[nt.id].reading = guess(nt.fields, READING_FIELDS);
    if (drafts[nt.id].target) preview(nt);
  }

  async function sync() {
    syncing = true;
    syncError = "";
    report = null;
    try {
      await api.setSetting("anki_query", query.trim());
      report = await api.ankiSync();
      onchange();
    } catch (e) {
      syncError = String(e);
    } finally {
      syncing = false;
    }
  }

  const dirty = (nt: NoteType) =>
    (nt.mapping?.target_field ?? "") !== drafts[nt.id]?.target || (nt.mapping?.reading_field ?? "") !== drafts[nt.id]?.reading;
  const mappedCount = $derived(noteTypes.filter((n) => n.mapping).length);
  const pct = (n: number, d: number) => (d ? Math.round((100 * n) / d) : 0);
</script>

<div class="page">
  <h1>Anki</h1>

  <section class="card">
    <h2>1. Connect</h2>
    <p class="muted">
      Ankura reads your cards through the AnkiConnect add-on (code <code>2055492159</code>). Anki has to be open while
      Ankura syncs. It only reads; it never changes your collection.
    </p>
    <div class="row">
      <input class="url" bind:value={url} placeholder="http://127.0.0.1:8765" />
      <button class="primary" onclick={connect} disabled={connecting}>{connecting ? "Connecting…" : "Connect"}</button>
      {#if version !== null}<span class="ok">✓ Connected (AnkiConnect v{version})</span>{/if}
    </div>
    {#if connectError}
      <p class="error">{connectError}</p>
      <p class="muted">Is Anki running, with AnkiConnect installed (Tools → Add-ons → Get Add-ons → 2055492159)?</p>
    {/if}
  </section>

  {#if version !== null}
    <section class="card">
      <h2>2. Which field holds the word?</h2>
      <p class="muted">
        For each note type with Japanese words, pick the field that holds the word, and the reading field if there is
        one. Preview shows what Ankura makes of a sample of your notes. Note types you leave unmapped are skipped.
      </p>
      {#if loadingTypes}<p class="muted">Loading note types…</p>{/if}
      <div class="types">
        {#each noteTypes as nt (nt.id)}
          {@const d = drafts[nt.id]}
          {#if d}
            <div class="nt" class:mapped={nt.mapping}>
              <div class="nt-head">
                <div>
                  <strong>{nt.name}</strong>
                  <span class="muted">{nt.notes.toLocaleString()} notes</span>
                  {#if nt.mapping}<span class="pill mapped-pill">mapped</span>{/if}
                </div>
                {#if !d.target}<button class="ghost" onclick={() => suggest(nt)}>Suggest fields</button>{/if}
              </div>
              <div class="row">
                <label>Word
                  <select bind:value={d.target}>
                    <option value="">— not mapped —</option>
                    {#each nt.fields as f}<option value={f}>{f}</option>{/each}
                  </select>
                </label>
                <label>Reading
                  <select bind:value={d.reading} disabled={!d.target}>
                    <option value="">— none —</option>
                    {#each nt.fields as f}<option value={f}>{f}</option>{/each}
                  </select>
                </label>
                <button onclick={() => preview(nt)} disabled={!d.target || d.busy}>{d.busy ? "Checking…" : "Preview"}</button>
                <button class="primary" onclick={() => save(nt)} disabled={!d.target || !dirty(nt)}>Save</button>
                {#if nt.mapping}<button class="ghost danger" onclick={() => unmap(nt)}>Unmap</button>{/if}
              </div>
              {#if d.error}<p class="error">{d.error}</p>{/if}
              {#if d.preview}
                {@const p = d.preview}
                <div class="preview">
                  <p>
                    Sample of {p.sampled} of {p.notes_total.toLocaleString()} notes:
                    <strong>{pct(p.words, p.sampled)}%</strong> single words,
                    {pct(p.phrases, p.sampled)}% phrases, {pct(p.empty, p.sampled)}% empty.
                  </p>
                  <table>
                    <tbody>
                      {#each p.examples as ex}
                        <tr class={ex.outcome}>
                          <td class="field" title={ex.field}>{ex.field.replace(/<[^>]*>/g, "").slice(0, 40) || "(empty)"}</td>
                          <td>→</td>
                          <td class="keys">
                            {#each ex.keys as k}<span class="key">{k.lemma}<small>{k.reading}</small></span>{/each}
                            {#if ex.outcome !== "word"}<span class="pill">{ex.outcome}</span>{/if}
                          </td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    </section>

    <section class="card">
      <h2>3. Sync</h2>
      <p class="muted">
        Optional: limit to part of your collection with an Anki search, for example <code>deck:日本語*</code>.
      </p>
      <div class="row">
        <input class="query" bind:value={query} placeholder="Anki search (optional)" />
        <button class="primary" onclick={sync} disabled={syncing || saving > 0 || mappedCount === 0}>{syncing ? "Syncing…" : "Sync now"}</button>
      </div>
      {#if syncError}<p class="error">{syncError}</p>{/if}
      {#if report}
        <p class="ok">
          Read {report.cards.toLocaleString()} cards from {report.notes.toLocaleString()} notes:
          {report.words.toLocaleString()} words, {report.phrases.toLocaleString()} phrases,
          {report.empty_fields.toLocaleString()} empty fields.
          {report.fsrs ? "FSRS is on, so stability and recall come from FSRS." : "FSRS is off, so intervals are used."}
        </p>
      {/if}
      {#if status && Object.keys(status.knowledge).length}
        <div class="row summary">
          {#each Object.entries(status.knowledge) as [state, n]}
            <span class="pill">{state.replace("_", " ")}: {n.toLocaleString()}</span>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .page {
    max-width: 920px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
  .url {
    width: 260px;
  }
  .query {
    flex: 1;
    min-width: 240px;
  }
  .ok {
    color: var(--accent);
  }
  code {
    background: var(--surface-2);
    padding: 0.05em 0.35em;
    border-radius: 4px;
  }
  .types {
    display: grid;
    gap: 0.7rem;
  }
  .nt {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.8rem 0.9rem;
    display: grid;
    gap: 0.55rem;
  }
  .nt.mapped {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--line));
  }
  .nt-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .nt-head strong {
    margin-right: 0.5rem;
  }
  .mapped-pill {
    margin-left: 0.4rem;
    color: var(--accent);
  }
  label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    font-size: 0.9rem;
    color: var(--muted);
  }
  .preview {
    background: var(--surface-2);
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    font-size: 0.9rem;
  }
  .preview p {
    margin: 0 0 0.4rem;
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  td {
    padding: 0.15rem 0.4rem;
    vertical-align: top;
  }
  td.field {
    font-family: var(--font-read);
    white-space: nowrap;
    max-width: 18rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .key {
    font-family: var(--font-read);
    margin-right: 0.6rem;
  }
  .key small {
    color: var(--muted);
    margin-left: 0.25rem;
  }
  tr.empty td.field {
    color: var(--danger);
  }
  .summary {
    margin-top: 0.6rem;
  }
</style>
