<script lang="ts">
  import { onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, type LibraryItem, type Status, type TextMeta } from "../lib/api";

  let { status, onopen, ongo }: {
    status: Status | null;
    onopen: (id: string) => void;
    ongo: (view: "anki" | "settings") => void;
  } = $props();

  let texts = $state<LibraryItem[]>([]);
  let query = $state("");
  type Sort = "recent" | "newest" | "oldest" | "title";
  const savedSort = (): Sort => {
    try {
      return (localStorage.getItem("ankura.library_sort") as Sort) || "newest";
    } catch {
      return "newest";
    }
  };
  let sort = $state<Sort>(savedSort());
  $effect(() => {
    try {
      localStorage.setItem("ankura.library_sort", sort);
    } catch {}
  });
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function search() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(load, 250);
  }

  /** A row: a text with its simpler versions, or a serial story with its episodes. */
  type Unit = { key: string; text: LibraryItem | null; title: string; children: LibraryItem[] };
  const units = $derived.by(() => {
    const ids = new Set(texts.map((t) => t.meta.id));
    const out: Unit[] = [];
    const series = new Map<string, Unit>();
    for (const t of texts) {
      if (t.parent && ids.has(t.parent)) continue; // listed under its original
      if (t.series) {
        let u = series.get(t.series);
        if (!u) {
          u = { key: `series:${t.series}`, text: null, title: t.series_title ?? "Serial story", children: [] };
          series.set(t.series, u);
          out.push(u);
        }
        u.children.push(t);
        continue;
      }
      out.push({ key: t.meta.id, text: t, title: t.meta.title, children: texts.filter((c) => c.parent === t.meta.id) });
    }
    for (const u of out) u.children.sort((a, b) => (a.part ?? a.episode ?? 0) - (b.part ?? b.episode ?? 0) || a.meta.created_at - b.meta.created_at);
    const members = (u: Unit) => (u.text ? [u.text, ...u.children] : u.children);
    const created = (u: Unit) => Math.max(...members(u).map((m) => m.meta.created_at));
    const first = (u: Unit) => Math.min(...members(u).map((m) => m.meta.created_at));
    const read = (u: Unit) => Math.max(...members(u).map((m) => m.meta.opened_at ?? m.meta.created_at));
    const by: Record<Sort, (a: Unit, b: Unit) => number> = {
      recent: (a, b) => read(b) - read(a),
      newest: (a, b) => created(b) - created(a),
      oldest: (a, b) => first(a) - first(b),
      title: (a, b) => a.title.localeCompare(b.title, "ja"),
    };
    return out.sort(by[sort]);
  });
  let title = $state("");
  let body = $state("");
  let error = $state("");
  let saving = $state(false);

  async function load() {
    try {
      texts = await api.library(query);
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  async function start() {
    if (!body.trim()) return;
    saving = true;
    try {
      const t = await api.addText(title, body);
      title = "";
      body = "";
      onopen(t.id);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function remove(t: TextMeta) {
    // (Simpler versions and episodes stay; they move up to the top level.)
    if (!(await ask(`Delete “${t.title}”?`, { title: "Delete text", kind: "warning" }))) return;
    await api.deleteText(t.id);
    await load();
  }

  const when = (ms: number) => new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });

  const setup = $derived(
    status
      ? [
          { done: status.tokenizer_ready, label: "Japanese tokenizer dictionary (SudachiDict)", go: "settings" as const },
          { done: status.dictionaries.length > 0, label: "Word dictionary (JMdict) for look-ups", go: "settings" as const },
          { done: status.mappings > 0, label: "Connect Anki and choose which fields hold your words", go: "anki" as const },
        ]
      : [],
  );
  const setupDone = $derived(setup.every((s) => s.done));
</script>

<div class="page">
  {#if status && !setupDone}
    <section class="card setup">
      <h2>Getting set up</h2>
      <ul>
        {#each setup as s}
          <li class:done={s.done}>
            <span class="check">{s.done ? "✓" : "○"}</span>
            <span>{s.label}</span>
            {#if !s.done}<button class="ghost" onclick={() => ongo(s.go)}>Set up →</button>{/if}
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section class="card new">
    <h2>New text</h2>
    <input class="title" placeholder="Title (optional)" bind:value={title} />
    <textarea
      placeholder="Paste a Japanese text here: an article, a chapter, song lyrics…"
      bind:value={body}
      rows="7"
      onkeydown={(e) => {
        if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) start();
      }}
    ></textarea>
    <div class="row">
      <button class="primary" disabled={!body.trim() || saving || !status?.tokenizer_ready} onclick={start}>Start reading</button>
      <span class="muted">{body.length ? `${body.length.toLocaleString()} characters` : "Ctrl+Enter also works"}</span>
    </div>
  </section>

  {#if error}<p class="error">{error}</p>{/if}

  <section>
    <div class="head">
      <h2>Your texts</h2>
      <input class="search" type="search" placeholder="Search titles and texts" bind:value={query} oninput={search} />
      <select bind:value={sort} aria-label="Order">
        <option value="newest">Newest first</option>
        <option value="oldest">Oldest first</option>
        <option value="recent">Recently read</option>
        <option value="title">Title</option>
      </select>
    </div>
    {#if texts.length === 0}
      <p class="muted">{query.trim() ? "No text contains that." : "Nothing here yet. Paste something above to start."}</p>
    {:else}
      <ul class="texts">
        {#each units as u (u.key)}
          <li class="unit">
            {#if u.text}
              {@const t = u.text}
              <div class="row-item">
                <button class="open" onclick={() => onopen(t.meta.id)}>
                  <span class="t-title">{t.meta.title}</span>
                  <span class="muted">
                    {#if t.parent}Simpler version {t.part} of {t.of} · {/if}{t.meta.chars.toLocaleString()} characters ·
                    {when(t.meta.created_at)}{u.children.length
                      ? ` · ${u.children.length} ${query.trim() ? "matching" : "simpler"} version${u.children.length === 1 ? "" : "s"}`
                      : ""}
                  </span>
                </button>
                <button class="ghost danger" title="Delete" onclick={() => remove(t.meta)}>Delete</button>
              </div>
            {:else}
              <div class="series-head">
                <span class="t-title">{u.title}</span>
                <span class="muted">serial story · {u.children.length} episode{u.children.length === 1 ? "" : "s"}</span>
              </div>
            {/if}
            {#if u.children.length}
              <ul class="children">
                {#each u.children as c (c.meta.id)}
                  <li class="row-item">
                    <button class="open child" onclick={() => onopen(c.meta.id)}>
                      <span class="t-title">{c.meta.title}</span>
                      <span class="muted">
                        {c.episode ? `Episode ${c.episode}` : `Simpler version ${c.part} of ${c.of}`} ·
                        {c.meta.chars.toLocaleString()} characters · {when(c.meta.created_at)}
                      </span>
                    </button>
                    <button class="ghost danger" title="Delete" onclick={() => remove(c.meta)}>Delete</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .page {
    max-width: 880px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .setup ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.4rem;
  }
  .setup li {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .setup li.done {
    color: var(--muted);
  }
  .check {
    width: 1.2rem;
    color: var(--accent);
    font-weight: 700;
  }
  .new {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .new textarea {
    resize: vertical;
    font-family: var(--font-read);
    font-size: 1.05rem;
    line-height: 1.8;
  }
  .texts {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.5rem;
  }
  .head {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 0.8rem;
  }
  .head h2 {
    margin: 0;
    flex: 1;
  }
  .search {
    min-width: 14rem;
  }
  .unit {
    list-style: none;
  }
  .row-item {
    display: flex;
    gap: 0.4rem;
    align-items: stretch;
  }
  .series-head {
    display: flex;
    flex-direction: column;
    padding: 0.4rem 0.2rem;
  }
  .children {
    list-style: none;
    margin: 0.35rem 0 0 1.4rem;
    padding: 0 0 0 0.8rem;
    border-left: 2px solid var(--line);
    display: grid;
    gap: 0.35rem;
  }
  .open.child {
    padding: 0.55rem 0.9rem;
  }
  .open.child .t-title {
    font-size: 0.98rem;
  }
  .open {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.15rem;
    padding: 0.8rem 1rem;
    text-align: left;
  }
  .t-title {
    font-weight: 600;
    font-family: var(--font-read);
    font-size: 1.05rem;
  }
</style>
