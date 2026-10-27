<script lang="ts">
  import { onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { api, type AiMode, type AiSettings, type CardOptions, type Provider, type Status } from "../lib/api";

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
  let mode = $state<AiMode>("chat");
  let key = $state("");
  let aiMessage = $state("");
  async function loadAi() {
    ai = await api.aiSettings();
    provider = ai.config.provider;
    model = ai.config.model;
    effort = ai.config.effort;
    mode = ai.mode;
  }
  async function switchMode() {
    await api.setSetting("ai_mode", mode);
    await loadAi();
  }
  // --- Anki cards (ADR 0012) ---
  let cardOpts = $state<CardOptions | null>(null);
  let cardsError = $state("");
  let cardNoteType = $state("");
  let cardDeck = $state("");
  let autoAdd = $state(false);
  let cardsMessage = $state("");
  async function loadCards() {
    cardsError = "";
    try {
      cardOpts = await api.cardOptions();
      cardNoteType = cardOpts.note_type ?? "";
      cardDeck = cardOpts.deck ?? "";
      autoAdd = cardOpts.auto_add;
    } catch (e) {
      cardsError = String(e);
    }
  }
  async function saveCards() {
    await api.setSetting("card_note_type", cardNoteType);
    await api.setSetting("card_deck", cardDeck);
    await api.setSetting("cards_auto_add", String(autoAdd));
    cardsMessage = "Saved.";
  }

  onMount(() => {
    loadAi().catch((e) => (error = String(e)));
    loadCards();
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
    <h2>AI</h2>
    <div class="grid">
      <label for="mode">How Ankura asks the AI</label>
      <select id="mode" bind:value={mode} onchange={switchMode}>
        <option value="api">Automatically, with an API key</option>
        <option value="chat">Copy &amp; paste through claude.ai</option>
      </select>
    </div>
    {#if mode === "chat"}
      <p class="muted">
        Ankura gives you a request to paste into claude.ai, and you paste Claude's reply back. No key needed; it uses
        your Claude subscription.
      </p>
    {:else}
      <p class="muted">
        Ankura asks the AI itself and checks the result. API use is billed per text (a few cents), separately from a
        Claude subscription, but Claude Max and Team plans include monthly API credits: claim them at
        console.anthropic.com and create a key there. The key is stored in your system's keychain, not in Ankura's
        files.
      </p>
    {/if}
    <div class="grid" class:hidden={mode === "chat"}>
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
    <div class="row" class:hidden={mode === "chat"}>
      <button class="primary" onclick={saveAi}>Save</button>
      {#if aiMessage}<span class="muted">{aiMessage}</span>{/if}
      {#if provider === "anthropic"}
        <span class="muted small">Get a key at console.anthropic.com → API keys.</span>
      {/if}
    </div>
  </section>

  <section class="card">
    <h2>Anki cards</h2>
    <p class="muted">
      Cards you make while reading go into this note type and deck. Ankura fills them in like your newest notes of
      that type, and only ever adds notes (tagged <code>ankura</code>).
    </p>
    {#if cardsError}
      <p class="error">{cardsError}</p>
      <button onclick={loadCards}>Try again</button>
    {:else if cardOpts}
      <div class="grid">
        <label for="cnt">Note type</label>
        <select id="cnt" bind:value={cardNoteType}>
          <option value="" disabled>Choose…</option>
          {#each cardOpts.note_types as n}<option value={n}>{n}</option>{/each}
        </select>
        <label for="cdk">Deck</label>
        <select id="cdk" bind:value={cardDeck}>
          <option value="" disabled>Choose…</option>
          {#each cardOpts.decks as d}<option value={d}>{d}</option>{/each}
        </select>
        <label for="caa">Add cards to Anki straight away, without the card list</label>
        <span><input id="caa" type="checkbox" bind:checked={autoAdd} /></span>
      </div>
      <div class="row">
        <button onclick={saveCards} disabled={!cardNoteType || !cardDeck}>Save</button>
        {#if cardsMessage}<span class="muted">{cardsMessage}</span>{/if}
      </div>
    {:else}
      <p class="muted">Asking Anki…</p>
    {/if}
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
  .hidden {
    display: none;
  }
</style>
