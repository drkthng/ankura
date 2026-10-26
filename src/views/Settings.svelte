<script lang="ts">
  import { onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { api, type AiSettings, type Provider, type Status } from "../lib/api";

  let { status, onchange }: { status: Status | null; onchange: () => void } = $props();

  let busy = $state<"" | "sudachi" | "jmdict" | "import">("");
  let error = $state("");
  let progress = $state<{ what: string; done: number; total: number | null } | null>(null);

  const unlisten = listen<{ what: string; done: number; total: number | null }>("download-progress", (e) => {
    progress = e.payload;
  });
  onDestroy(() => unlisten.then((f) => f()));

  async function run(kind: typeof busy, f: () => Promise<unknown>) {
    busy = kind;
    error = "";
    progress = null;
    try {
      await f();
      onchange();
    } catch (e) {
      error = String(e);
    } finally {
      busy = "";
      progress = null;
    }
  }

  const chooseSudachi = async () => {
    const path = await open({ title: "Choose SudachiDict system.dic", filters: [{ name: "Sudachi dictionary", extensions: ["dic"] }] });
    if (typeof path === "string") await run("sudachi", () => api.setSudachiPath(path));
  };
  const importDict = async () => {
    const path = await open({ title: "Choose a Yomitan dictionary", filters: [{ name: "Yomitan dictionary", extensions: ["zip"] }] });
    if (typeof path === "string") await run("import", () => api.importDictionary(path));
  };

  let knownDays = $state("");
  let fadingBelow = $state("");
  let countSuspended = $state(false);
  $effect(() => {
    if (status && knownDays === "") {
      knownDays = String(status.settings.known_days);
      fadingBelow = String(status.settings.fading_below);
      countSuspended = status.settings.count_suspended;
    }
  });
  async function saveKnowledge() {
    await api.setSetting("known_days", knownDays);
    await api.setSetting("fading_below", fadingBelow);
    await api.setSetting("count_suspended", String(countSuspended));
    onchange();
  }

  const mb = (n: number) => (n / 1048576).toFixed(1);

  // --- AI provider ---
  let ai = $state<AiSettings | null>(null);
  let provider = $state<Provider>("anthropic");
  let model = $state("");
  let effort = $state("medium");
  let key = $state("");
  let aiMessage = $state("");
  async function loadAi() {
    ai = await api.aiSettings();
    provider = ai.config.provider;
    model = ai.config.model;
    effort = ai.config.effort;
  }
  onMount(() => {
    loadAi().catch((e) => (error = String(e)));
  });
  async function switchProvider() {
    await api.setSetting("ai_provider", provider);
    await api.setSetting("ai_model", "");
    await loadAi();
  }
  async function saveAi() {
    aiMessage = "";
    try {
      await api.setSetting("ai_provider", provider);
      await api.setSetting("ai_model", model.trim());
      await api.setSetting("ai_effort", effort);
      if (key.trim()) {
        await api.setApiKey(provider, key.trim());
        key = "";
      }
      await loadAi();
      aiMessage = "Saved.";
    } catch (e) {
      aiMessage = String(e);
    }
  }
  async function removeKey() {
    await api.setApiKey(provider, "");
    await loadAi();
  }
</script>

<div class="page">
  <h1>Settings</h1>

  {#if error}<p class="error">{error}</p>{/if}
  {#if progress}
    <p class="muted">
      Downloading {progress.what}: {mb(progress.done)} MB{progress.total ? ` of ${mb(progress.total)} MB` : ""}…
    </p>
  {/if}

  <section class="card">
    <h2>Japanese tokenizer</h2>
    <p class="muted">
      Ankura splits Japanese text into words with Sudachi. It needs the SudachiDict core dictionary (about 77 MB to
      download, 190 MB on disk).
    </p>
    {#if status?.tokenizer_ready}
      <p class="ok">✓ Ready <span class="muted">({status.sudachi_path})</span></p>
    {/if}
    <div class="row">
      <button class="primary" onclick={() => run("sudachi", api.downloadSudachi)} disabled={busy !== ""}>
        {busy === "sudachi" ? "Downloading…" : status?.tokenizer_ready ? "Download again" : "Download SudachiDict"}
      </button>
      <button onclick={chooseSudachi} disabled={busy !== ""}>Use a system.dic I have…</button>
    </div>
  </section>

  <section class="card">
    <h2>Dictionaries</h2>
    <p class="muted">Word look-ups use Yomitan-format dictionaries. JMdict (English) is the standard one.</p>
    {#if status?.dictionaries.length}
      <ul>
        {#each status.dictionaries as d}
          <li>{d.title} <span class="muted">· {d.terms.toLocaleString()} entries{d.revision ? ` · ${d.revision}` : ""}</span></li>
        {/each}
      </ul>
    {/if}
    <div class="row">
      <button class="primary" onclick={() => run("jmdict", api.downloadJmdict)} disabled={busy !== ""}>
        {busy === "jmdict" ? "Downloading…" : "Download JMdict"}
      </button>
      <button onclick={importDict} disabled={busy !== ""}>{busy === "import" ? "Importing…" : "Import a Yomitan dictionary…"}</button>
    </div>
  </section>

  <section class="card">
    <h2>AI for “Write for me”</h2>
    <p class="muted">
      Texts are written by an AI model you pay for with your own API key. A short text costs a few cents. The key is
      stored in your system's keychain, not in Ankura's files.
    </p>
    <div class="grid">
      <label for="prov">Provider</label>
      <select id="prov" bind:value={provider} onchange={switchProvider}>
        <option value="anthropic">Anthropic (Claude)</option>
        <option value="open_router">OpenRouter (many models)</option>
      </select>
      <label for="key">API key {#if ai?.has_key}<span class="ok">✓ saved</span>{/if}</label>
      <span class="row">
        <input
          id="key"
          type="password"
          autocomplete="off"
          placeholder={ai?.has_key ? "•••••••• (enter a new key to replace)" : provider === "anthropic" ? "sk-ant-…" : "sk-or-…"}
          bind:value={key}
        />
        {#if ai?.has_key}<button class="ghost danger" onclick={removeKey}>Remove</button>{/if}
      </span>
      <label for="model">Model</label>
      <input
        id="model"
        bind:value={model}
        placeholder={provider === "anthropic" ? "claude-opus-5-5" : "e.g. anthropic/… (see openrouter.ai/models)"}
      />
      {#if provider === "anthropic"}
        <label for="effort">Effort <span class="muted">(more = better texts, slower, pricier)</span></label>
        <select id="effort" bind:value={effort}>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      {/if}
    </div>
    <div class="row">
      <button class="primary" onclick={saveAi}>Save</button>
      {#if aiMessage}<span class="muted">{aiMessage}</span>{/if}
      {#if provider === "anthropic"}
        <span class="muted small">Get a key at console.anthropic.com → API keys.</span>
      {/if}
    </div>
  </section>

  <section class="card">
    <h2>When is a word “known”?</h2>
    <div class="grid">
      <label for="kd">Known at a stability (FSRS) or interval of at least</label>
      <span><input id="kd" type="number" min="1" bind:value={knownDays} /> days</span>
      <label for="fb">Show as fading when Anki's recall estimate is below</label>
      <span><input id="fb" type="number" min="0" max="1" step="0.05" bind:value={fadingBelow} /></span>
      <label for="cs">Count suspended cards</label>
      <span><input id="cs" type="checkbox" bind:checked={countSuspended} /></span>
    </div>
    <div class="row">
      <button onclick={saveKnowledge}>Save</button>
      <span class="muted">Takes effect at the next Anki sync.</span>
    </div>
  </section>

  {#if status}<p class="muted small">Data folder: {status.data_dir}</p>{/if}
</div>

<style>
  .page {
    max-width: 820px;
    margin: 0 auto;
    padding: 2rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
  .ok {
    color: var(--accent);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.6rem 1rem;
    align-items: center;
    margin-bottom: 0.8rem;
  }
  .grid input[type="number"] {
    width: 6rem;
  }
  .grid input#key,
  .grid input#model {
    width: 22rem;
  }
  ul {
    margin: 0 0 0.8rem;
    padding-left: 1.2rem;
  }
  .small {
    font-size: 0.8rem;
  }
</style>
