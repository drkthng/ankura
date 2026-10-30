<script lang="ts">
  import { onMount } from "svelte";
  import { api, stateLabel, type ReaderToken, type Simplified, type WordInfo, type WordState } from "../lib/api";
  import KanjiPanel from "./KanjiPanel.svelte";
  import * as speech from "../lib/speech";

  let {
    token,
    anchor,
    onclose,
    onmark,
    ondontknow,
    sentence = "",
    textId = null,
    onreading = () => {},
    onlisten,
    context = "",
  }: {
    token: ReaderToken;
    anchor: DOMRect;
    onclose: () => void;
    onmark: (state: WordState | null) => void;
    ondontknow: () => void;
    /** The sentence the word appears in, for a card. */
    sentence?: string;
    textId?: string | null;
    /** A reading was corrected: the text needs reloading. */
    onreading?: () => void;
    /** Read the text aloud from this word's sentence (ADR 0017). */
    onlisten?: () => void;
    /** The paragraph around the sentence, for a translation that fits (ADR 0020). */
    context?: string;
  } = $props();

  let info = $state<WordInfo | null>(null);

  // --- meanings in English or in simple Japanese (ADR 0020) ---
  const pref = (): "en" | "ja" => {
    try {
      return localStorage.getItem("ankura.meaning_lang") === "ja" ? "ja" : "en";
    } catch {
      return "en";
    }
  };
  let lang = $state<"en" | "ja">(pref());
  let ja = $state<{ text: string; tokens: ReaderToken[]; coverage: number } | null>(null);
  let jaBusy = $state(false);
  let jaError = $state("");
  const JA = /[\u3040-\u30ff]/;
  /** Entries from dictionaries that explain in Japanese (a monolingual Yomitan dictionary). */
  const jaEntries = $derived((info?.entries ?? []).filter((e) => e.senses.length && e.senses.filter((s) => JA.test(s)).length * 2 >= e.senses.length));
  const enEntries = $derived((info?.entries ?? []).filter((e) => !jaEntries.includes(e)));
  async function setLang(l: "en" | "ja") {
    lang = l;
    try {
      localStorage.setItem("ankura.meaning_lang", l);
    } catch {}
    if (l === "ja") loadJa();
    requestAnimationFrame(place);
  }
  async function loadJa() {
    if (ja || jaBusy || !token.key) return;
    jaBusy = true;
    jaError = "";
    try {
      ja = await api.wordExplainJa(token.key, token.dictionary_form, sentence);
      requestAnimationFrame(place);
    } catch (e) {
      jaError = String(e);
    } finally {
      jaBusy = false;
    }
  }

  // --- the sentence translated in its context ---
  let translation = $state<{ translation: string; word_note: string } | null>(null);
  let translating = $state(false);
  let translateError = $state("");
  async function translate() {
    translating = true;
    translateError = "";
    try {
      translation = await api.sentenceTranslate(sentence, context, token.key, token.dictionary_form);
      requestAnimationFrame(place);
    } catch (e) {
      translateError = String(e);
    } finally {
      translating = false;
    }
  }

  // --- listening: the word from its reading in kana, so a lone kanji isn't misread ---
  let speakError = $state("");
  async function speakWord() {
    speakError = "";
    try {
      await speech.say(token.key?.reading || token.dictionary_form, "narrator", textId);
    } catch (e) {
      speakError = String(e).replace(/^Error: /, "");
    }
  }

  // --- correcting the reading ---
  let fixing = $state(false);
  let fixReading = $state("");
  let fixScope = $state<"text" | "everywhere">("text");
  let fixError = $state("");
  const shownReading = $derived(token.segments.map((g) => g.ruby ?? g.text).join(""));
  async function saveReading(reading: string | null) {
    fixError = "";
    try {
      await api.setReadingFix(token.surface, reading, fixScope === "text" ? textId : null);
      onreading();
    } catch (e) {
      fixError = String(e);
    }
  }

  // --- kanji (ADR 0013) ---
  const KANJI = /[\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]/u;
  const kanjiChars = $derived([...new Set([...token.dictionary_form].filter((c) => KANJI.test(c)))]);
  let openKanji = $state<string | null>(null);

  // --- "Simplify this sentence" ---
  let simple = $state<Simplified | null>(null);
  let simplifying = $state(false);
  let simpleError = $state("");
  let showTranslation = $state(false);
  async function simplify() {
    simplifying = true;
    simpleError = "";
    try {
      simple = await api.simplifySentence(sentence);
      requestAnimationFrame(place);
    } catch (e) {
      simpleError = String(e);
    } finally {
      simplifying = false;
    }
  }
  const familiar = (t: ReaderToken) => ["known", "fading", "marked_known", "ignored"].includes(t.state);

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

  let error = $state("");
  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  onMount(() => {
    if (token.key) {
      api.wordInfo(token.key, token.dictionary_form).then((i) => (info = i), (e) => (error = String(e)));
      if (lang === "ja") loadJa();
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
  const best = $derived(enEntries[0]);
  const others = $derived(enEntries.slice(1, 4));
  const isMarked = $derived(token.source === "mark");
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} onresize={place} />

<div class="backdrop" role="presentation" onclick={onclose}></div>
<div class="pop" bind:this={el} style:left="{pos.left}px" style:top="{pos.top}px" role="dialog" aria-label="Word details">
  <header>
    <div class="word">
      <span class="lemma">{token.dictionary_form}</span>
      <button class="speak" title="Listen" aria-label="Listen to the word" onclick={speakWord}>🔊</button>
      {#if key.reading && key.reading !== token.dictionary_form}
        <span class="reading">
          {key.reading}
          {#if token.has_kanji}
            <button class="fix" title="Wrong reading? Correct it" onclick={() => { fixing = !fixing; fixReading = shownReading; requestAnimationFrame(place); }}>✎</button>
          {/if}
        </span>
      {/if}
      {#if key.lemma !== token.dictionary_form}<span class="also">also written {key.lemma}</span>{/if}
    </div>
    <span class="state st-{token.state}">{token.source === "parts" ? "Known parts" : token.source === "basic" ? "Basic word" : stateLabel[token.state]}</span>
  </header>

  {#if fixing}
    <section class="fixer">
      <p class="muted small">Reading of <strong class="jp">{token.surface}</strong> here:</p>
      {#if info?.readings.length}
        <div class="chips">
          {#each info.readings as r}<button class="chip" class:on={fixReading === r} onclick={() => (fixReading = r)}>{r}</button>{/each}
        </div>
      {/if}
      <div class="row">
        <input class="jp" bind:value={fixReading} aria-label="Reading in kana" />
        <select bind:value={fixScope} aria-label="Where the reading applies">
          <option value="text">in this text</option>
          <option value="everywhere">everywhere</option>
        </select>
      </div>
      <div class="row">
        <button class="primary" disabled={!fixReading.trim()} onclick={() => saveReading(fixReading)}>Use this reading</button>
        <button class="ghost" onclick={() => saveReading(null)} title="Back to the reading Ankura worked out">Reset</button>
      </div>
      {#if fixError}<p class="error small">{fixError}</p>{/if}
    </section>
  {/if}

  {#if error}<p class="error">{error}</p>{/if}

  <div class="langs" role="tablist" aria-label="Meaning in">
    <button role="tab" class:on={lang === "en"} aria-selected={lang === "en"} onclick={() => setLang("en")}>English</button>
    <button role="tab" class:on={lang === "ja"} aria-selected={lang === "ja"} onclick={() => setLang("ja")}>日本語</button>
  </div>

  {#if lang === "ja"}
    <section class="ja-meaning">
      {#each jaEntries.slice(0, 2) as e}
        <p class="jp"><strong>{e.expression}</strong> 【{e.reading}】 {e.senses.slice(0, 3).join(" / ")}</p>
      {/each}
      {#if ja}
        <p class="jp explain">
          {#each ja.tokens as t}{#each t.segments as g}{#if g.ruby && t.key && !familiar(t)}<ruby>{g.text}<rt>{g.ruby}</rt></ruby>{:else}{g.text}{/if}{/each}{/each}
        </p>
        <p class="muted small">やさしい説明 (AI, for this sentence) · {Math.round(ja.coverage * 100)}% words you know</p>
      {:else if jaBusy}
        <p class="muted">やさしい説明を書いています…</p>
      {/if}
      {#if jaError}<p class="error small">{jaError}</p>{/if}
    </section>
  {/if}

  {#if info && lang === "en"}
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
  {/if}
  {#if info}
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

  {#if kanjiChars.length}
    <div class="kanji-row">
      <span class="muted small">Kanji:</span>
      {#each kanjiChars as k}
        <button class="kchip" class:on={openKanji === k} onclick={() => (openKanji = openKanji === k ? null : k)}>{k}</button>
      {/each}
    </div>
    {#if openKanji}<KanjiPanel kanji={openKanji} onresize={place} />{/if}
  {/if}

  {#if sentence}
    <section class="simplify">
      {#if simple}
        <p class="simple">
          {#each simple.tokens as t}{#each t.segments as g}{#if g.ruby && t.key && !familiar(t)}<ruby>{g.text}<rt>{g.ruby}</rt></ruby>{:else}{g.text}{/if}{/each}{/each}
        </p>
        <p class="muted small">
          Simpler version · {Math.round(simple.coverage * 100)}% words you know ·
          {#if showTranslation}{simple.translation}{:else}<button class="link" onclick={() => (showTranslation = true)}>Show translation</button>{/if}
        </p>
      {:else}
        <button class="ghost" disabled={simplifying} onclick={simplify} title={sentence}>
          {simplifying ? "Simplifying…" : "Simplify this sentence"}
        </button>
      {/if}
      {#if simpleError}<p class="error small">{simpleError}</p>{/if}
      {#if translation}
        <p class="translation">{translation.translation}</p>
        {#if translation.word_note}<p class="muted small">Here, <strong class="jp">{token.dictionary_form}</strong>: {translation.word_note}</p>{/if}
      {:else}
        <button class="ghost" disabled={translating} onclick={translate} title="A translation that fits the text around it">
          {translating ? "Translating…" : "Translate this sentence"}
        </button>
      {/if}
      {#if translateError}<p class="error small">{translateError}</p>{/if}
      {#if onlisten}<button class="ghost" onclick={onlisten} title="Read the text aloud from this sentence">▶ Listen from here</button>{/if}
      {#if speakError}<p class="error small">{speakError}</p>{/if}
    </section>
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
  .langs {
    display: inline-flex;
    align-self: flex-start;
    width: fit-content;
    border: 1px solid var(--line);
    border-radius: 999px;
    overflow: hidden;
    margin: 0.2rem 0 0.4rem;
  }
  .langs button {
    border: none;
    border-radius: 0;
    padding: 0.15em 0.9em;
    font-size: 0.8rem;
    background: none;
  }
  .langs button.on {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .ja-meaning .explain {
    font-size: 1.05rem;
    line-height: 1.9;
    margin: 0.2rem 0;
  }
  .translation {
    margin: 0.4rem 0 0.2rem;
  }
  .speak {
    border: none;
    background: none;
    padding: 0 0.2rem;
    cursor: pointer;
    font-size: 0.95rem;
  }
  .fix {
    border: none;
    background: none;
    padding: 0 0.2rem;
    color: var(--muted);
    cursor: pointer;
    font-size: 0.85rem;
  }
  .fixer {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.5rem 0.6rem;
    display: grid;
    gap: 0.4rem;
  }
  .fixer .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .fixer .chip {
    font-family: var(--font-read);
    padding: 0.1em 0.6em;
    border-radius: 999px;
    font-size: 0.95rem;
  }
  .fixer .chip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .fixer input {
    width: 9rem;
  }
  .jp {
    font-family: var(--font-read);
  }
  .kanji-row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .kchip {
    font-family: var(--font-read);
    font-size: 1.05rem;
    padding: 0.05em 0.5em;
  }
  .kchip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .simplify {
    border-top: 1px solid var(--line);
    padding-top: 0.5rem;
  }
  .simplify > button {
    font-size: 0.85rem;
    padding: 0.2em 0.6em;
  }
  .simple {
    font-family: var(--font-read);
    font-size: 1.15rem;
    line-height: 2;
    margin: 0 0 0.2rem;
  }
  .simple rt {
    font-size: 0.5em;
    color: var(--muted);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font-size: inherit;
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
