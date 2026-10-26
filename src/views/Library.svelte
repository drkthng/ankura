<script lang="ts">
  import { onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, type Status, type TextMeta } from "../lib/api";

  let { status, onopen, ongo }: {
    status: Status | null;
    onopen: (id: string) => void;
    ongo: (view: "anki" | "settings") => void;
  } = $props();

  let texts = $state<TextMeta[]>([]);
  let title = $state("");
  let body = $state("");
  let error = $state("");
  let saving = $state(false);

  async function load() {
    try {
      texts = await api.listTexts();
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
    <h2>Your texts</h2>
    {#if texts.length === 0}
      <p class="muted">Nothing here yet. Paste something above to start.</p>
    {:else}
      <ul class="texts">
        {#each texts as t (t.id)}
          <li>
            <button class="open" onclick={() => onopen(t.id)}>
              <span class="t-title">{t.title}</span>
              <span class="muted">{t.chars.toLocaleString()} characters · {when(t.opened_at ?? t.created_at)}</span>
            </button>
            <button class="ghost danger" title="Delete" onclick={() => remove(t)}>Delete</button>
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
  .texts li {
    display: flex;
    gap: 0.4rem;
    align-items: stretch;
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
