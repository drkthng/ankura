# Architecture options

Status: proposal, 2026-10-26. Built on `docs/research/landscape.md` and
`docs/research/claims-check.md`. The owner's decisions on the open questions are in `docs/adr/`.

## 1. What the architecture has to support

- One app on Windows, macOS, Linux and Android, offline-first, with the same reader on both.
- A Japanese tokenizer on the device (phones too): lemma, reading and part of speech per token.
- Read Anki scheduling state: AnkiConnect on desktop, AnkiDroid on the phone.
- A large, old Anki collection with many note types. Field mapping has to be configured, not guessed.
- Ankura's own data (exposures, lookups, word statuses, texts, interests) on both devices.
- LLM generation with a deterministic check of the result.
- A reading UI that is pleasant to use. For Japanese that means furigana (`<ruby>`), optional
  vertical text, tap-to-look-up, and popovers.

## 2. Getting Anki data (HANDOFF §5, first question)

### Desktop: AnkiConnect

1. **Card selection:** the user picks decks and/or writes an Anki search (default: the mapped
   note types, `-is:suspended`). `findCards(query)` returns card IDs.
2. **Card data:** `cardsInfo` for interval, type, queue, reps, lapses, factor and fields.
   `cardsModTime` on later runs, so only changed cards are fetched again (asbplayer's approach).
3. **FSRS state:** `cardsInfo` doesn't include it (claims-check #7). Two ways to get it:
   - **Search buckets (default):** run `findCards` with ranges such as `prop:r>=0.9`,
     `prop:r>=0.7`, `prop:s>=21`, `prop:s>=90`. Each card lands in an R bucket and an S bucket.
     About ten queries per refresh, whatever the collection size. Needs Anki 23.10+ with FSRS on;
     when `prop:s>=0` returns nothing, FSRS is off and Ankura uses the interval.
   - **Exact mode (optional):** read the newest automatic backup
     (`<profile>/backups/backup-*.colpkg`). It's a zip holding a zstd-compressed SQLite file, and
     its `cards.data` JSON has `s`, `d`, `decay` and `lrt`, so R can be computed exactly. Safe
     because Anki never touches a backup once written. Its data is up to 30 minutes old by
     default.
4. **Later option:** a small Ankura add-on for Anki that exposes the memory state over HTTP. That's
   more code to maintain, so it waits until the search buckets turn out to be too coarse.

### Android: three paths, in order of preference

**(P) AnkiDroid ContentProvider directly (primary).** This option wasn't in HANDOFF §5, because
the prior report didn't know the provider could do this.

- Since AnkiDroid 2.24.0 (May 2026), `content://com.ichi2.anki.flashcards/cards` takes an Anki
  search as `selection`. It returns per-card `interval`, `lapses`, `reps`, `type`, `queue`,
  `fsrs_stability`, `fsrs_difficulty`, `fsrs_decay` and `last_review_time_secs`. That's enough to
  compute R exactly on the phone. Notes and fields come from
  `content://com.ichi2.anki.flashcards/notes`, which also accepts an Anki search.
- Needs the runtime permission `com.ichi2.anki.permission.READ_WRITE_DATABASE` and a `<queries>`
  manifest entry. Works offline. It always shows the phone's own up-to-date view of Anki, which
  matters because the user may review on the phone.
- **Incremental refresh:** query `rated:N` and `added:N` (N = days since the last refresh) instead
  of the whole collection. A full scan only on first run or when the field mapping changes.
- **Risks:** the user may have AnkiDroid older than 2.24, or may deny the permission. Query speed on
  a large collection hasn't been measured (the provider loads rows lazily). Spike S2 below
  measures it.

**(a) Knowledge snapshot from the desktop through Ankura's own sync (fallback).** The desktop
computes per-lemma knowledge from AnkiConnect and puts it in the sync stream. The phone shows it.
- Works with any AnkiDroid version, and without AnkiDroid at all.
- It's only as fresh as the last desktop run, and reviews done on the phone only show up after
  AnkiWeb sync, a desktop refresh, and an Ankura sync. It costs little, because the desktop computes
  the same data anyway. Build it once Ankura's own sync exists.

**(c) Import a `.colpkg`/`.apkg` export (fallback, both platforms).** The user exports a
collection package from AnkiDroid or Anki, or picks a desktop backup, and Ankura reads the cards
and notes tables. LWT already does this, including the zstd `collection.anki21b` format.
- Works offline and anywhere, and gives exact FSRS state.
- It's manual and goes stale. Good for onboarding and as an escape hatch, not for daily use.

**(b) Anki's sync server or protocol: don't.** The protocol is internal and changes with Anki
releases. The manual says the server is not a data API and third-party implementations "are not
recommended". A user who self-hosts the official server could point Ankura at the stored
`collection.anki2` read-only, but that's a niche setup and not worth building for v1.

**Proposal:** P as the Android default, (c) as the fallback in v1, and (a) once sync exists.
Desktop: AnkiConnect with search buckets, plus optional exact mode from backups.

## 3. Knowledge model and note-to-lemma mapping (HANDOFF §5, second question)

### Field mapping (per note type)

A setup screen lists every note type the selected decks use, with a note count and three sample
notes. For each note type the user picks:

| Setting | Example | Notes |
|---|---|---|
| Target field | `Expression`, `Vocab`, `Front` | The word or kanji the card teaches. Required, or "ignore this note type". |
| Kind | word / kanji / sentence | Sentence cards feed every token in the sentence at a lower weight. |
| Reading field | `Reading` | Optional. Used to tell homographs apart (e.g. 今日 きょう vs こんにち). |
| Context field | `Sentence` | Optional. Tokens here count as "seen in Anki", at lower weight (VocabSieve's idea). |
| Which cards count | all / only the recognition template | A note with production and recognition cards: the user picks which card templates count. |

The screen shows a live preview of the normalized lemma for the sample notes, and how many notes
of that type failed to normalize, with a link to the failures. Mappings are stored by note type
ID, so renaming a note type doesn't lose them. Ankura proposes defaults based on field names and
known templates (Core 2k/6k, Kaishi 1.5k, Yomitan's default fields), but nothing is applied until
the user accepts.

### Normalizing a field value

A fixed, tested pipeline, run on the same input on both platforms (so it lives in the shared core):

1. Strip HTML (keep text inside `<ruby>`'s base, drop `<rt>`), decode entities, drop `[sound:…]`
   and image tags.
2. Strip Anki furigana markup: `漢字[かんじ]` → `漢字`, keeping the bracket part as a reading
   candidate. Strip cloze markup `{{c1::…::hint}}` → inner text.
3. Normalize Unicode (NFKC for width only, no kana folding), trim, collapse whitespace.
4. Split on list separators (`、`, `,`, `;`, `/`, `・` when between whole words) when the field
   holds several forms.
5. Tokenize each part. One content token: take its lemma (dictionary form). Several tokens: try
   the whole string as a dictionary entry first (compounds, set phrases), otherwise record it as
   a multi-token expression.
6. Kanji set = every character in the CJK Unified Ideographs blocks.
7. Key = (language, lemma, reading or empty). The reading comes from the reading field if mapped,
   otherwise from the tokenizer.

Each card links to one or more keys, and failures go in a review list instead of being guessed.

### From card state to word state

Per card, at the time the reader opens a text:

- `R` = FSRS retrievability now, from stability, decay and last review time (exact on Android and
  in exact mode, bucketed via `prop:r` otherwise). SM-2 cards: estimate from the interval.
- `S` = stability, or the interval for SM-2 cards.

Per key: take the card with the **highest** S (AnkiMorphs' rule, so one strong card is enough).
Then:

| State | Rule (defaults, all configurable) |
|---|---|
| Unknown | No mapped card, and not marked in Ankura. |
| New in Anki | Card exists, never reviewed. Counts as "supposed to know soon" (target word). |
| Learning | Reviewed, S < 21 days. |
| Known | S ≥ 21 days. |
| Fading | Known, but R < 0.8 today. Shown with a subtle marker; good material for generated texts. |
| Suspended | Cards that are only suspended don't count as known, unless the user opts in. |

Kanji state: from a mapped kanji deck if there is one; otherwise a kanji is known when it appears
in at least 3 known words (configurable).

### Ankura's own signal

- **Exposure:** a token counts as seen once its paragraph has been visible for 3 seconds (one
  count per token per reading session).
- **Lookup:** the user tapped "I don't know this" or opened the popover for more than a quick
  glance.
- **Friction** = lookups / exposures over the last 30 days. A key that Anki says is known but has
  friction > 0.3 with at least 5 exposures is a **stealth leech**. It's shown as such and fed into
  generated texts.
- Manual overrides always win: "I know this" or "ignore" (names, foreign words) on a single word.

## 4. Build vs integrate (HANDOFF §5, third question)

**Fork or extend Lute (MIT).**
- For: mature term model (statuses, multi-word terms, parent terms), parser plugins, an active
  community, AnkiConnect export, MIT licence.
- Against: it's a Python/Flask server rendered in a browser. On Android that means Termux or a
  phone browser pointed at the desktop, which conflicts with "works seamlessly on Android". It
  reads nothing from Anki. Both features Ankura is defined by (Anki knowledge, AI generation)
  would be new code in a codebase built around other assumptions, and the upstream project might
  not want them.
- A fair version of this option: contribute "read Anki stats" to Lute upstream, and build only the
  Android app. That splits the product across two stacks and two projects.

**Build on LinguaCafe (GPL-3.0).**
- For: a Japanese reading UI with Sudachi and JMdict, AnkiConnect push.
- Against: GPL would force Ankura's licence to change. The stack (Laravel, MySQL, Redis, Python
  spaCy service, Docker, x64 only, over 2 GB of RAM) can't run on a phone. `main` has been quiet
  since March 2025 while a rewrite happens on a branch. **Not recommended.**

**Start fresh (MIT), integrate through protocols and formats.**
- For: designed for local-first on two platforms from day one. Anki knowledge and LLM generation
  are central, not bolted on. Keeps MIT. It can still reuse MIT/BSD code (FSRS libraries) and
  interoperate: AnkiConnect, the AnkiDroid provider, Yomitan dictionaries, AnkiMorphs known-morphs
  CSV, Lute term CSV export, `.apkg`/`.colpkg`.
- Against: everything has to be built, including the parts Lute already has (term editing, text
  library, multi-word terms). The reader UX is the part that takes most of the effort, and nothing
  is reused there.

**Recommendation: start fresh**, with imports from Lute and AnkiMorphs so existing users of both
can bring their data. The tools that ran on Anki data on Android (jidoujisho) and on desktop
(AnkiMorphs) were both just archived, so Ankura isn't duplicating a maintained project.

## 5. Sync of Ankura's own data (HANDOFF §5, fourth question)

### Data model first

The data is mergeable by construction, whatever the transport:

| Data | Merge rule |
|---|---|
| Exposure and lookup counts | Per-device counters (a G-counter): each device only increments its own row, and the total is the sum. Never conflicts. |
| Event log (exposure, lookup, status change, with timestamp and device ID) | Append-only per device, union on merge. Counters can be rebuilt from it. |
| Word status overrides, interest weights, settings | Last writer wins per key, by hybrid logical clock. |
| Texts (pasted or generated) | Immutable, keyed by UUID; edits create a new version. |
| Anki knowledge snapshot | Written only by the desktop; the phone just reads it. |

Each device keeps its own SQLite database and exports its changes since the last sync as a
compressed batch file. A sync applies the other device's batches. No library is needed for this.
Automerge (active, Rust 0.12) is the step up if collaborative editing of something like a text
ever comes up.

### Transport options

| Option | For | Against |
|---|---|---|
| **LAN pairing:** desktop and phone pair once by QR code, then sync over the local network when both are open | No accounts, no server, no third party. The phone can pull the desktop's Anki snapshot at the same time. | Only syncs at home or when both devices are on the same network. Needs discovery (mDNS) and a pairing flow. |
| **Shared folder** (Syncthing-Fork, Dropbox, Google Drive, OneDrive) | Uses whatever the user already has. Each device writes its own batch files. | On Android, folder access goes through the Storage Access Framework, which needs plugin work in any stack. The official Syncthing Android app was discontinued in December 2024 (Syncthing-Fork continues, outside Play). |
| **WebDAV** (Nextcloud, many NAS boxes, some hosted services) | Plain HTTPS, works from anywhere, easy on Android. | The user needs a WebDAV server. |
| **Hosted sync server** | Seamless. | Accounts, hosting cost, privacy and operations. Not for a personal tool at v1. |

Server sync engines (ElectricSQL, PowerSync, Turso) need a backend; cr-sqlite is dormant. None of
them is recommended (landscape and stack notes).

**Recommendation:** the batch-file model above, with **LAN pairing** as the first transport and
WebDAV or a shared folder as the second.

## 6. LLM provider and generation pipeline (HANDOFF §5, fifth question)

- **Providers:** a native Anthropic client and one OpenAI-compatible client. The OpenAI-compatible
  one covers OpenRouter (`https://openrouter.ai/api/v1`) and local Ollama (`/v1`, which supports
  `response_format` with a JSON schema). Default models: `claude-sonnet-5-5` for generation,
  `claude-haiku-5-5` for cheap tasks (simplify a sentence, extract interests). The user brings
  their own key, stored in the OS keychain (Android Keystore on the phone).
- **Structured output:** request a JSON schema on every path (Anthropic structured outputs,
  OpenAI-style `response_format`), and validate it on the client anyway, because support varies
  between OpenRouter providers.
- **Request:** target keys (new words or kanji), the user's level, a sample of known lemmas and
  fading words for context, interests (weighted), length, and difficulty level. The response
  holds title, text, and per-target example sentences with translations.
- **Deterministic check:**
  1. Tokenize the result with the same tokenizer as the reader.
  2. Coverage = (tokens that are known, learning, or a requested target) / (all content tokens).
     Punctuation, numbers, and names the tokenizer tags as proper nouns don't count.
  3. Check that every target appears at least once.
  4. If coverage < threshold (default 0.95) or a target is missing: send the model back the list of
     offending lemmas and ask for a rewrite. At most two retries.
  5. Still too low: keep the text, but pre-gloss the remaining unknowns, and show the real coverage
     in the preview bar.
- Results are cached by input hash, so regenerating costs nothing until the inputs change.

## 7. Language scope (HANDOFF §5, sixth question)

Japanese first, with a language-agnostic core:

- Every key and text carries a language code.
- The tokenizer sits behind an interface (`tokenize(text) -> [Token{surface, lemma, reading?,
  pos, start, end}]`), with Japanese as the first implementation.
- Script features are optional per language: kanji tracking and furigana for Japanese, nothing for
  space-delimited languages.
- A second language later means a new tokenizer and dictionary, not a schema change. Lute's parser
  plugins (jieba, Thai, Khmer) and its space-delimited parser show how little a basic
  space-delimited tokenizer needs.

The cost is small now and large to add later, so build it this way even if v1 only does Japanese.

## 8. Stack options

All three options share the same core idea: one shared core (tokenizer, normalization, knowledge
model, coverage check, sync batches, SQLite) used by both platforms, and a thin native layer on
Android for the AnkiDroid provider.

### Option A: Tauri v2, Rust core, web UI (Svelte or React)

- **Core:** Rust. Lindera (MIT, 6.2.0) with IPADIC (47.5 MB unpacked) or UniDic as the first
  tokenizer, rusqlite, an HTTP client for AnkiConnect and the LLMs, fsrs-rs (BSD-3) if needed.
- **UI:** a web front end. The browser engine already handles what Japanese reading needs:
  `<ruby>` furigana, `writing-mode: vertical-rl`, font fallback, text selection, and fine-grained
  typography. ttu, Mokuro Reader and AJATT Reader show how far that goes.
- **Android:** a Kotlin Tauri plugin for the AnkiDroid provider (`activity.contentResolver`). The
  plugin API takes arbitrary permission strings.
- **Proof it works:** Readest ships an e-book reader on Tauri v2 to Android and iOS. ReadBridge
  uses Tauri 2 on desktop.
- **Risks:** 91 open Android issues, among them UI-thread deadlocks, a lost Activity callback that
  hangs an invoke (relevant for the permission prompt), keyboard and viewport bugs, and the back
  button. Tauri 3 is in alpha, so a migration is coming. Linux uses WebKitGTK, the weakest of the
  three web engines.

### Option B: Kotlin Multiplatform + Compose Multiplatform

- **Core:** Kotlin. Sudachi Java (`com.worksap.nlp:sudachi` 0.8.2, maintained) or Lucene's Kuromoji
  (maintained; Atilika Kuromoji is dead since 2019). SQLDelight or Room. AnkiDroid access is
  native, with no bridge.
- **UI:** Compose on both. Android is first-class; the desktop runs on the JVM (Compose MP 1.12.2).
- **Risks:** Compose has no ruby or vertical text layout. Furigana has to be laid out by hand
  (stacked `Text`s or a custom layout), and vertical Japanese is a large custom job. That's the
  reader, the most important screen. Desktop installers carry a JVM, so they're tens of MB.
  SudachiDict is 115-202 MB, and Lucene Kuromoji ties you to Lucene's analyzer API.

### Option C: Flutter, with a Rust core via flutter_rust_bridge

- **Core:** Rust as in option A, called through `flutter_rust_bridge` 2.13. Alternatively MeCab via
  `mecab_for_flutter`, as jidoujisho did.
- **UI:** Flutter 3.47 on Android and desktop. AnkiDroid through a platform channel, or the
  existing `ankidroid_for_flutter` 1.0.4.
- **Risks:** Flutter has no ruby or vertical text either; furigana means custom rendering
  (jidoujisho did this). Desktop plugin coverage is thinner than mobile. Three languages (Dart,
  Rust, Kotlin) instead of two.

### Comparison

| | A: Tauri | B: KMP/Compose | C: Flutter + Rust |
|---|---|---|---|
| Japanese text rendering (ruby, vertical) | Built into the engine | Custom work | Custom work |
| Android integration (AnkiDroid) | Kotlin plugin, some bridge risk | Native | Platform channel, easy |
| Android platform maturity | Weakest of the three | Strongest | Strong |
| Desktop | Good (small installers; WebKitGTK on Linux) | Good (JVM) | OK |
| Shared core | Rust | Kotlin | Rust |
| Tokenizer options | Lindera, Vibrato, Sudachi.rs | Sudachi Java, Lucene Kuromoji | Same as A, or MeCab via FFI |
| Languages to maintain | Rust, TS, a bit of Kotlin | Kotlin | Dart, Rust, a bit of Kotlin |
| App size (excluding dictionary) | Small | Large on desktop (JVM) | Medium |

### Recommendation: Option A (Tauri v2 + Rust core + Svelte)

The reader is the product, and it's a Japanese reader. Web rendering gives furigana and vertical
text for free, and the owner wants the app to be fun and easy, which is mostly UI work. Tauri's
Android weakness touches only a small part of the app (the AnkiDroid plugin, the back button, the
keyboard on a few screens). Two spikes check it before any commitment (below). If the spikes go
badly, the Rust core moves to option C unchanged. That's the fallback.

Svelte over React: smaller output, and simpler to write fine-grained reactive token spans. The
reading tools nearest to Ankura (ttu, Mokuro Reader, AJATT Reader, Lukaisu) are Svelte. Either
works.

## 9. Spikes before scaffolding

These are throwaway tests, run after the owner approves the architecture:

| ID | Question | Pass condition |
|---|---|---|
| S1 | Tauri Android hello-world + AnkiDroid plugin: does the permission prompt work, does a `cards` query return FSRS columns? | Rows with `fsrs_stability` from a real AnkiDroid 2.25 install. |
| S2 | AnkiDroid query speed on a large collection (the owner's, or a 50k-card test collection). | Full scan under 30 s, incremental `rated:N` under 2 s. |
| S3 | Tokenizer comparison on 200 real sentences: Lindera+IPADIC, Lindera+UniDic, Sudachi core. | Lemma and reading accuracy; furigana derivable; APK size measured. |
| S4 | Field normalization on an export of the owner's collection: what share of notes per note type normalize cleanly? | Above 95% on the main note types, with failures listed. |
| S5 | One generation round with the coverage check, against a real known-word set. | Coverage ≥ 0.95 within two retries on most requests. |
