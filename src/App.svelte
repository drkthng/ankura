<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Status } from "./lib/api";
  import Library from "./views/Library.svelte";
  import Reader from "./views/Reader.svelte";
  import AnkiSetup from "./views/AnkiSetup.svelte";
  import Settings from "./views/Settings.svelte";
  import Write from "./views/Write.svelte";
  import Cards from "./views/Cards.svelte";
  import StatsView from "./views/Stats.svelte";
  import KanjiView from "./views/Kanji.svelte";
  import TestView from "./views/Test.svelte";
  import Practice from "./views/Practice.svelte";
  import Guide from "./views/Guide.svelte";
  import Today from "./views/Today.svelte";

  type View = { name: "today" } | { name: "library" } | { name: "reader"; id: string; today?: boolean } | { name: "write" } | { name: "cards" } | { name: "stats" } | { name: "kanji" } | { name: "test" } | { name: "practice" } | { name: "guide" } | { name: "anki"; focus?: string } | { name: "settings" };

  let view = $state<View>({ name: "today" });

  // A text opened from Today shows the session's steps (ADR 0019).
  let todayDone = $state<string[]>([]);
  async function openFromToday(id: string) {
    todayDone = (await api.todayOverview().catch(() => null))?.session.done ?? [];
    view = { name: "reader", id, today: true };
  }
  let status = $state<Status | null>(null);
  let statusError = $state("");

  async function refresh() {
    try {
      status = await api.status();
      statusError = "";
    } catch (e) {
      statusError = String(e);
    }
  }

  onMount(refresh);

  const knownWords = $derived(
    status ? (status.knowledge.known ?? 0) + (status.knowledge.fading ?? 0) : 0,
  );
</script>

<div class="shell">
  <nav>
    <div class="brand">
      <span class="logo" aria-hidden="true">🌱</span>
      <span>Ankura</span>
    </div>
    <button class:active={view.name === "today" || (view.name === "reader" && view.today)} onclick={() => (view = { name: "today" })}>Today</button>
    <button class:active={view.name === "library" || (view.name === "reader" && !view.today)} onclick={() => (view = { name: "library" })}>
      Library
    </button>
    <button class:active={view.name === "write"} onclick={() => (view = { name: "write" })}>Write for me</button>
    <button class:active={view.name === "cards"} onclick={() => (view = { name: "cards" })}>Cards</button>
    <button class:active={view.name === "stats"} onclick={() => (view = { name: "stats" })}>Stats</button>
    <button class:active={view.name === "kanji"} onclick={() => (view = { name: "kanji" })}>Kanji</button>
    <button class:active={view.name === "test"} onclick={() => (view = { name: "test" })}>Level check</button>
    <button class:active={view.name === "practice"} onclick={() => (view = { name: "practice" })}>Practice test</button>
    <button class:active={view.name === "anki"} onclick={() => (view = { name: "anki" })}>Anki</button>
    <button class:active={view.name === "guide"} onclick={() => (view = { name: "guide" })}>How to learn</button>
    <button class:active={view.name === "settings"} onclick={() => (view = { name: "settings" })}>Settings</button>
    <div class="spacer"></div>
    {#if status}
      <div class="nav-stat" title="Words Anki says you know">
        <strong>{knownWords.toLocaleString()}</strong>
        <span>known words</span>
      </div>
    {/if}
  </nav>

  <main>
    {#if statusError}
      <p class="error">{statusError}</p>
    {/if}
    {#if view.name === "today"}
      <Today onopen={openFromToday} ongo={(name) => (view = { name })} />
    {:else if view.name === "library"}
      <Library {status} onopen={(id) => (view = { name: "reader", id })} ongo={(name) => (view = { name })} />
    {:else if view.name === "reader"}
      {#key view.id}
        <Reader
          id={view.id}
          onback={() => (view = { name: view.name === "reader" && view.today ? "today" : "library" })}
          oncards={() => (view = { name: "cards" })}
          onopen={(id: string) => (view = { name: "reader", id })}
          today={view.today
            ? {
                done: todayDone,
                onstep: async (s: string) => {
                  todayDone = (await api.todayMark(s)).done;
                },
              }
            : null}
        />
      {/key}
    {:else if view.name === "write"}
      <Write onopen={(id) => (view = { name: "reader", id })} ongo={(name) => (view = { name })} />
    {:else if view.name === "cards"}
      <Cards ongo={(name) => (view = { name })} />
    {:else if view.name === "stats"}
      <StatsView />
    {:else if view.name === "kanji"}
      <KanjiView ongo={() => (view = { name: "anki", focus: "kanji" })} />
    {:else if view.name === "test"}
      <TestView />
    {:else if view.name === "practice"}
      <Practice />
    {:else if view.name === "guide"}
      <Guide />
    {:else if view.name === "anki"}
      <AnkiSetup {status} onchange={refresh} focus={view.focus} />
    {:else if view.name === "settings"}
      <Settings {status} onchange={refresh} />
    {/if}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100%;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 1.1rem 0.8rem;
    background: var(--surface);
    border-right: 1px solid var(--line);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 700;
    font-size: 1.2rem;
    padding: 0.2rem 0.6rem 1rem;
  }
  .logo {
    font-size: 1.3rem;
  }
  nav button {
    text-align: left;
    border: none;
    background: transparent;
    padding: 0.55rem 0.75rem;
  }
  nav button:hover {
    background: var(--surface-2);
  }
  nav button.active {
    background: var(--surface-2);
    font-weight: 600;
    color: var(--accent);
  }
  .spacer {
    flex: 1;
  }
  .nav-stat {
    display: flex;
    flex-direction: column;
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    background: var(--surface-2);
  }
  .nav-stat strong {
    font-size: 1.3rem;
  }
  .nav-stat span {
    color: var(--muted);
    font-size: 0.8rem;
  }
  main {
    overflow: auto;
    min-width: 0;
  }
</style>
