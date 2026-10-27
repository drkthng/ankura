<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, type PrimitiveRow, type StoryCandidate, type StoryRewrite } from "../lib/api";

  let { ongo }: { ongo: (view: "anki") => void } = $props();

  let status = $state<{ with_story: number; analysed: number } | null>(null);
  let rows = $state<PrimitiveRow[]>([]);
  /** Approved names being edited, per component. */
  let keep = $state<Record<string, string[]>>({});
  let custom = $state<Record<string, string>>({});
  let showAll = $state(false);
  let error = $state("");
  let analysing = $state(false);
  let progress = $state<{ done: number; total: number } | null>(null);
  let savedList = $state("");

  let candidates = $state<StoryCandidate[]>([]);
  let rewrites = $state<StoryRewrite[]>([]);
  let pick = $state<Record<string, boolean>>({});
  let preparing = $state(false);
  let saving = $state(false);
  let results = $state<Record<string, string>>({});

  const unlisten = listen<{ done: number; total: number }>("primitives-progress", (e) => (progress = e.payload));
  onDestroy(() => unlisten.then((f) => f()));

  async function load() {
    error = "";
    try {
      status = await api.primitivesStatus();
      rows = await api.primitivesList();
      keep = Object.fromEntries(rows.map((r) => [r.component, [...r.approved]]));
      candidates = await api.primitivesCandidates();
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  async function analyse(again = false) {
    analysing = true;
    progress = null;
    error = "";
    try {
      if (again) await api.primitivesReset();
      status = await api.primitivesAnalyse();
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      analysing = false;
    }
  }

  function toggle(c: string, name: string) {
    const cur = keep[c] ?? [];
    keep = { ...keep, [c]: cur.includes(name) ? cur.filter((n) => n !== name) : [...cur, name] };
  }
  function addName(c: string) {
    const n = (custom[c] ?? "").trim().toLowerCase();
    if (!n) return;
    keep = { ...keep, [c]: [...new Set([...(keep[c] ?? []), n])] };
    custom = { ...custom, [c]: "" };
  }

  async function saveList() {
    error = "";
    try {
      await api.primitivesApprove(Object.fromEntries(Object.entries(keep).filter(([, ns]) => ns.length)));
      savedList = "Saved. New mnemonics use these names from now on.";
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function prepare() {
    preparing = true;
    error = "";
    try {
      // A first batch to review; the rest after saving these.
      rewrites = await api.primitivesRewrite(candidates.slice(0, 24).map((c) => c.kanji));
      pick = Object.fromEntries(rewrites.map((r) => [r.kanji, true]));
      results = {};
    } catch (e) {
      error = String(e);
    } finally {
      preparing = false;
    }
  }

  async function saveChosen() {
    saving = true;
    for (const r of rewrites.filter((r) => pick[r.kanji])) {
      try {
        const n = await api.primitivesSave(r.kanji, r.new);
        results = { ...results, [r.kanji]: `✓ saved (${n} ${n === 1 ? "note" : "notes"})` };
      } catch (e) {
        results = { ...results, [r.kanji]: String(e) };
      }
    }
    saving = false;
    status = await api.primitivesStatus();
    candidates = await api.primitivesCandidates();
  }

  const text = (html: string) => {
    const d = document.createElement("div");
    d.innerHTML = html.replace(/<br\s*\/?>/gi, "\n");
    return d.textContent ?? "";
  };
  const clashes = $derived(rows.filter((r) => r.clash).length);
  const shown = $derived(showAll || !clashes ? rows : rows.filter((r) => r.clash));
  const todo = $derived(status ? status.with_story - status.analysed : 0);
</script>

<div class="page">
  <h1>Kanji</h1>
  <p class="muted lead">
    Make the names of primitives consistent across all your kanji stories: one name per component, or two where the
    component really is two things (月 as moon, and as flesh in body-part kanji). Ankura reads your stories, shows which
    names you use and where they clash, and after you approve the list, rewrites the stories that use other names.
    You check every story before it goes into Anki, and each change can be undone from the kanji pop-up.
  </p>
  {#if error}<p class="error">{error}</p>{/if}

  {#if status && status.with_story === 0}
    <section class="card notice">
      <p>No kanji stories yet. Map your kanji note types and sync first.</p>
      <button class="primary" onclick={() => ongo("anki")}>Go to Anki</button>
    </section>
  {:else if status}
    <section class="card">
      <h2>1. Read your stories</h2>
      <p>
        {status.analysed.toLocaleString()} of {status.with_story.toLocaleString()} stories analysed.
        {#if todo && !analysing}<span class="muted">The AI reads about 40 stories per request.</span>{/if}
      </p>
      {#if analysing}
        <div class="progress"><div style:width="{progress ? (100 * progress.done) / Math.max(1, progress.total) : 2}%"></div></div>
        <div class="row">
          <span class="muted small">{progress ? `${progress.done} of ${progress.total}` : "Starting…"}</span>
          <button class="ghost" onclick={() => api.primitivesStop()}>Stop after this batch</button>
        </div>
      {:else}
        <div class="row">
          {#if todo}<button class="primary" onclick={() => analyse()}>Analyse {todo.toLocaleString()} {todo === status.analysed ? "" : "new or changed "}stories</button>{/if}
          {#if status.analysed}<button class="ghost" onclick={() => analyse(true)}>Analyse all again</button>{/if}
        </div>
      {/if}
    </section>

    {#if rows.length}
      <section class="card">
        <h2>2. Primitive names</h2>
        <p class="muted small">
          Tick the names to keep for each component. {clashes} {clashes === 1 ? "component has" : "components have"} more
          than one name. RTK's name is shown for reference.
        </p>
        {#if clashes}<label class="small"><input type="checkbox" bind:checked={showAll} /> Show all {rows.length} components</label>{/if}
        <div class="prims">
          {#each shown as r (r.component)}
            <div class="prim" class:clash={r.clash}>
              <span class="comp">{r.component}</span>
              <div class="names">
                {#each r.names as n}
                  <label class="name" title={n.kanji.join(" ")}>
                    <input type="checkbox" checked={keep[r.component]?.includes(n.name)} onchange={() => toggle(r.component, n.name)} />
                    {n.name} <span class="muted">×{n.count}</span>
                  </label>
                {/each}
                {#each (keep[r.component] ?? []).filter((k) => !r.names.some((n) => n.name === k)) as extra}
                  <label class="name"><input type="checkbox" checked onchange={() => toggle(r.component, extra)} /> {extra}</label>
                {/each}
                <input class="add" placeholder="other name" bind:value={custom[r.component]} onkeydown={(e) => e.key === "Enter" && addName(r.component)} />
                {#if r.rtk_name && !(keep[r.component] ?? []).includes(r.rtk_name)}
                  <button class="link small" onclick={() => toggle(r.component, r.rtk_name)}>+ RTK: {r.rtk_name}</button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
        <div class="row">
          <button class="primary" onclick={saveList}>Save list</button>
          {#if savedList}<span class="muted small">{savedList}</span>{/if}
        </div>
      </section>
    {/if}

    {#if rows.some((r) => r.is_approved)}
      <section class="card">
        <h2>3. Update stories</h2>
        {#if rewrites.length}
          <p class="muted small">Untick any you'd rather keep. Saving changes only the mnemonic field, and each can be undone.</p>
          {#each rewrites as r (r.kanji)}
            <div class="rw">
              <label><input type="checkbox" bind:checked={pick[r.kanji]} disabled={!!results[r.kanji]} /> <span class="k">{r.kanji}</span></label>
              <div>
                <p class="old">{text(r.old)}</p>
                <p>{text(r.new)}</p>
                {#if results[r.kanji]}<p class="small" class:error={!results[r.kanji].startsWith("✓")}>{results[r.kanji]}</p>{/if}
              </div>
            </div>
          {/each}
          <div class="row">
            <button class="primary" disabled={saving || !rewrites.some((r) => pick[r.kanji] && !results[r.kanji])} onclick={saveChosen}>
              {saving ? "Saving…" : `Save ${rewrites.filter((r) => pick[r.kanji] && !results[r.kanji]).length} to Anki`}
            </button>
            <button class="ghost" disabled={saving} onclick={() => (rewrites = [])}>Done</button>
          </div>
        {:else if !candidates.length}
          <p class="muted">Every analysed story uses approved names.</p>
        {:else}
          <p>{candidates.length} {candidates.length === 1 ? "story uses" : "stories use"} a name that isn't on your list.</p>
          <ul class="cands">
            {#each candidates.slice(0, 8) as c}
              <li><span class="k">{c.kanji}</span> {c.keyword}: {c.replacements.map((r) => `${r.old} → ${r.new.join(" / ")}`).join(", ")}</li>
            {/each}
            {#if candidates.length > 8}<li class="muted">and {candidates.length - 8} more</li>{/if}
          </ul>
          <button class="primary" disabled={preparing} onclick={prepare}>
            {preparing ? "Writing new versions…" : `Prepare new versions${candidates.length > 24 ? " (first 24)" : ""}`}
          </button>
        {/if}
      </section>
    {/if}
  {:else if !error}
    <p class="muted">Loading…</p>
  {/if}
</div>

<style>
  .page {
    max-width: 920px;
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
  .progress {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-2);
    overflow: hidden;
    margin: 0.4rem 0;
  }
  .progress div {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }
  .prims {
    display: grid;
    gap: 0.4rem;
    margin: 0.7rem 0;
  }
  .prim {
    display: flex;
    gap: 0.8rem;
    align-items: baseline;
    padding: 0.35rem 0.5rem;
    border-radius: 8px;
  }
  .prim.clash {
    background: var(--surface-2);
  }
  .comp {
    font-family: var(--font-read);
    font-size: 1.4rem;
    min-width: 2rem;
  }
  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem 0.9rem;
    align-items: center;
  }
  .name {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }
  .add {
    width: 8rem;
    font-size: 0.85rem;
    padding: 0.15em 0.4em;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
  }
  .cands {
    margin: 0.4rem 0 0.8rem;
    padding-left: 1.2rem;
  }
  .k {
    font-family: var(--font-read);
    font-size: 1.2rem;
  }
  .rw {
    display: grid;
    grid-template-columns: 4rem 1fr;
    gap: 0.6rem;
    padding: 0.5rem 0;
    border-top: 1px solid var(--line);
  }
  .rw p {
    margin: 0 0 0.2rem;
    white-space: pre-line;
  }
  .rw .old {
    color: var(--muted);
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--muted) 50%, transparent);
  }
</style>
