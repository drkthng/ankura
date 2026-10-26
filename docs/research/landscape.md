# Landscape: tools relevant to Ankura

Researched 2026-10-26. Each open-source entry was checked against the repo itself: a shallow clone
to read LICENSE, README, CHANGELOG and source, plus the GitHub page for the archived banner. Nothing
from the clones was run. File paths in brackets are inside the named repo.

Some sites could not be reached from the research environment: jpdb.io, lingq.com, migaku.com,
satorireader.com, readlang.com, languagereactor.com, ankiweb.net, git.sr.ht, openrouter.ai,
v2.tauri.app. Facts about those come from search results and help or forum pages, and are marked
*(secondary)*. The closed-source section is lower confidence than the rest.

## Summary

| Tool | Repo | License | Last commit / release | Archived | Stack | Platforms | Anki link |
|---|---|---|---|---|---|---|---|
| Lute v3 | github.com/LuteOrg/lute-v3 | MIT | 2026-09-04 / 3.10.3 (2026-07) | No | Python, Flask, SQLite | Desktop (local web server) | AnkiConnect, export only |
| LWT (community) | github.com/HugoFara/learning-with-texts | Unlicense | 2026-09-04 / 3.7.0 | No | PHP 8, MySQL, TS + Alpine | Self-hosted web | `.apkg` export and import |
| Lukaisu | github.com/lukaisu/lukaisu | MIT | 2026-07-04 / none | No | Capacitor 8, Svelte 5, Dexie, ts-fsrs | Android | None |
| LinguaCafe | github.com/simjanos-dev/LinguaCafe | GPL-3.0 | main 2025-03, `vue3` branch 2026-08 / v0.15-beta | No | Laravel, Vue, MySQL, Redis, Python spaCy | Self-hosted web, x64 only | AnkiConnect, push only |
| ReadBridge | github.com/WindChimeEcho/read-bridge | MIT | 2026-09-18 / v0.1.11 | No | Next.js, React, Tauri 2 | Web, desktop | None |
| AnkiMorphs | github.com/mortii/anki-morphs | AGPL-3.0 | 2026-10-23 / v6.4.0 | **Yes, 2026-10-23** | Python Anki add-on | Anki desktop | Inside Anki (direct DB) |
| MorphMan | github.com/kaegi/MorphMan | GPL/AGPL | 2023-08-13 | No banner, inactive | Python Anki add-on | Anki desktop | Inside Anki |
| jidoujisho | github.com/arianneorpilla/jidoujisho | GPL-3.0 | 2025-10-20 / 2.9.1 | **Yes, 2026-10-10** | Flutter, Java channels | Android | AnkiDroid AddContentApi, write only |
| Yomitan | github.com/yomidevs/yomitan | GPL-3.0 | 2026-10-22 / 26.10.12.0 | No | WebExtension (JS) | Desktop browsers, Firefox Android | AnkiConnect |
| asbplayer | github.com/asbplayer/asbplayer | AGPL-3.0 | 2026-10-25 / v1.22.0 | No | TypeScript, React, WXT | Web, extension, partial Android | AnkiConnect (reads card status) |
| VocabSieve | github.com/FreeLanguageTools/vocabsieve | GPL-3.0 | 2025-08-15 / v0.12.5 | No | Python, PyQt5 | Desktop | AnkiConnect (reads known words) |
| Memento | github.com/ripose-jp/Memento | GPL-2.0 | 2026-08-23 / v2.0.2 | No | C++, Qt6, libmpv, MeCab | Desktop | AnkiConnect, write only |
| ttu reader | github.com/ttu-ttu/ebook-reader | BSD-3 | 2026-10-18 / v2.0.0 (2021) | No (maintenance only) | SvelteKit PWA | Web | None (uses Yomitan) |
| AJATT Reader | github.com/Ajatt-Tools/ajatt_book_reader | AGPL-3.0 | active 2026-10 | No | Svelte | Web | None |
| mokuro / Mokuro Reader | github.com/kha-white/mokuro, github.com/Gnathonic/mokuro-reader | GPL-3.0 | 2026-07 / 2026-10 | No | Python CLI / SvelteKit PWA | Desktop CLI / Web | Reader: AnkiConnect |
| KOReader + anki.koplugin | github.com/koreader/koreader, github.com/Ajatt-Tools/anki.koplugin | AGPL-3.0 | 2026-10 / 2026-08 | No | Lua | E-readers, Android, Linux | AnkiConnect over LAN, offline queue |
| Jiten | github.com/Sirush/Jiten | Apache-2.0 | 2026-10-24 / none | No | .NET 9, Sudachi.rs, Nuxt | Web (jiten.moe) | Known-word import via AnkiConnect *(secondary)* |
| Readest | github.com/readest/readest | AGPL-3.0 | 2026-10-25 / v0.12.16 | No | Next.js, Tauri v2 | Desktop, Android, iOS, web | None |
| AnkiConnect | git.sr.ht/~foosoft/anki-connect (GitHub mirror archived 2025-11-04) | GPL-3.0+ | GitHub 2025-11-03 | GitHub mirror yes | Python Anki add-on | Anki desktop | Is the link |
| AnkiconnectAndroid | github.com/KamWithK/AnkiconnectAndroid | GPL-3.0 | 2026-02-21 / 1.15 | No | Java Android app | Android | AnkiDroid provider, 15 actions |
| AnkiDroid | github.com/ankidroid/Anki-Android | GPL-3.0 (app), LGPL (`api`) | 2026-10-25 / v2.25.3 stable, v2.26.0alpha8 | No | Kotlin, Anki Rust backend | Android | Is the link (ContentProvider) |
| Anki sync server | github.com/ankitects/anki (`rslib/sync`) | AGPL-3.0+ | ships with Anki 26.09.3 | No | Rust | Any | Is Anki's sync |
| FSRS libraries | github.com/open-spaced-repetition | BSD-3 (fsrs-rs), MIT (py-fsrs, ts-fsrs) | 2026-10 | No | Rust, Python, TS | Libraries | n/a |

**Licensing:** Ankura is MIT. Almost everything above is GPL or AGPL. Ankura can talk to those
tools over their protocols (AnkiConnect HTTP, the AnkiDroid ContentProvider, the Yomitan API), and
read their documented data formats (Yomitan dictionaries, AnkiMorphs CSV, `.apkg`/`.colpkg`), but it
must not copy their code. The MIT/BSD FSRS libraries, Lute (MIT), Lukaisu (MIT), ReadBridge (MIT),
Jiten (Apache-2.0) and LWT (public domain) are the exceptions where code reuse is possible.

**Two gaps opened this month:** AnkiMorphs (the main "Anki tells me which words I know, give me
i+1" engine) and jidoujisho (the main Android immersion app with AnkiDroid support) were both
archived in October 2026. Nothing maintained combines Anki-derived known words with a reader on
Android.

---

## 1. Readers with word tracking

### Lute v3

- **Repo:** github.com/LuteOrg/lute-v3. `jzohrab/lute-v3` redirects to it (same HEAD SHA). Ships
  on PyPI as `lute3`.
- **License:** MIT. **Activity:** HEAD 2026-09-04, tag 3.10.3 (2026-07-07), many outside
  contributors in the CHANGELOG.
- **Stack:** Python 3.8+, Flask, Flask-SQLAlchemy, waitress, SQLite. Parsers: built-in
  space-delimited, character, and MeCab via `natto-py` [`lute/parse/mecab_parser.py`]; plugins for
  Mandarin (jieba), Thai and Khmer [`plugins/`]. **No spaCy.** The user installs MeCab separately.
- **Term model:** statuses 1-5, 98 Ignored, 99 Well known [`lute/models/term.py:315-317`],
  multi-word terms, parent terms, a `wordsread` table for reading stats [`lute/models/book.py:261`].
- **Anki:** since 3.10.0 (2025-02), "flexible Anki export" through AnkiConnect, with export specs
  (deck, note type, field mapping, selection criteria) [`lute/ankiexport/`]. One way. It never
  reads Anki stats. The manual says it doesn't work from a phone.
- **Mobile:** none. The manual suggests opening the desktop server from a phone browser, or
  running it under Termux.
- **For Ankura:** borrow the export-spec design (deck + note type + field mapping + criteria) and
  the parser plugin interface. Avoid "desktop server plus phone browser" as the mobile story.
  The licence allows code reuse, but the Python/Flask server doesn't fit an offline Android app.

### LWT community fork and Lukaisu

- **LWT:** github.com/HugoFara/learning-with-texts (`HugoFara/lwt` redirects). Public domain
  (Unlicense). Very active: 3.7.0 on 2026-09-04. PHP 8.2+, MySQL/MariaDB, TypeScript + Alpine.js.
  MeCab and jieba helpers in Python.
  - **Anki:** `.apkg` export and import, not AnkiConnect. The importer reads `collection.anki21b`
    (zstd) and **replays each card's revlog through LWT's own FSRS-6 scheduler**, so it can
    round-trip without a sync protocol (CHANGELOG #264, #238). Deletions don't round-trip.
  - Also: coverage preview per text before importing, Gutenberg books ranked by difficulty.
- **Lukaisu:** github.com/lukaisu/lukaisu, MIT. A Capacitor 8 app (Svelte 5, TypeScript, Dexie,
  ts-fsrs) wrapped for Android, **not Kotlin**. Local-first; an optional LWT-compatible server
  adds CJK segmentation, TTS and discovery. Without the server, Japanese is split character by
  character. No Anki link. The README says it isn't on F-Droid or Play yet; LWT's ROADMAP says it
  has its own F-Droid repo. The two disagree.
- **For Ankura:** the revlog-replay import is a good model for the backup-import fallback.
  Lukaisu shows the cost of having no on-device Japanese tokenizer: Ankura needs one on the phone.

### LinguaCafe

- github.com/simjanos-dev/LinguaCafe, GPL-3.0. `main` has been quiet since 2025-03-19; a `vue3`
  rewrite branch was active in 2026-08.
- Laravel 11, Vue 2 + Vuetify, MySQL 8, Redis, and a Python spaCy tokenizer service in Docker.
  Japanese via spaCy `ja_core_news_sm` (built on SudachiPy) plus pykakasi. JMdict import. x64 only;
  over 2 GB of RAM with all languages (README).
- **Anki:** AnkiConnect `addNote`, `updateNoteFields`, `findNotes`. Push only.
- **For Ankura:** a reference for a Japanese reading UI on Sudachi + JMdict. Its server stack
  can't run on Android, and the GPL rules out code reuse.

### ReadBridge

- github.com/WindChimeEcho/read-bridge, MIT. HEAD 2026-09-18, v0.1.11. Next.js 14, React 18, Dexie,
  Tauri 2 (desktop only, no Android config). **Tauri, not Electron** as the prior report said.
- What it does: you read your own books sentence by sentence and an LLM explains each one in the
  target language at a level you pick. It does **not** generate texts, track known words or talk
  to Anki. LLM calls go through the Vercel AI SDK, so any OpenAI-compatible endpoint works.
- **For Ankura:** a small reference for provider wiring and "explain in the target language"
  prompts. Not a competitor on known words or Anki.

### Jiten (jiten.moe)

- github.com/Sirush/Jiten, Apache-2.0, HEAD 2026-10-24. An open jpdb-style site: per-work
  difficulty, character counts, frequency lists, prebuilt decks and per-user coverage. .NET 9,
  Sudachi.rs, a JMdict-based deconjugator, Nuxt 4. Imports known words from Anki through
  AnkiConnect *(secondary: its guide page via search)*. Derived data is CC BY-SA 4.0
  *(secondary)*.
- **For Ankura:** frequency data and coverage-per-work presentation. The licence allows reuse.

## 2. Reading front-ends (Japanese)

- **ttu ebook reader** (github.com/ttu-ttu/ebook-reader, BSD-3): SvelteKit PWA, EPUB/HTMLZ/TXT,
  vertical text, furigana settings, character-count statistics. Maintenance only. No Anki; uses
  Yomitan for lookups.
- **AJATT Reader** (github.com/Ajatt-Tools/ajatt_book_reader, AGPL): calls itself the successor to
  ttu, with the same feature set.
- **mokuro** (GPL-3.0): an offline OCR CLI for manga that writes selectable text overlays.
  **Mokuro Reader** moved to github.com/Gnathonic/mokuro-reader (GPL-3.0, v1.10.0). It has
  AnkiConnect card creation with a configurable URL, plus reading stats and goals.
- **Readest** (github.com/readest/readest, AGPL-3.0): an e-book reader on Next.js + Tauri v2 that
  ships to desktop, **Android and iOS**, and imports Yomitan dictionaries. No Anki. It shows that
  Tauri v2 can carry a reading app to Android.
- **For Ankura:** borrow the Japanese reading conventions: vertical and horizontal text, furigana
  toggles, character-count progress and reading speed (the community's standard metrics), and
  goals. Manga OCR is out of scope for v1.

## 3. Anki-side tools

### AnkiMorphs (archived)

- github.com/mortii/anki-morphs, AGPL-3.0. **Archived 2026-10-23**; the README says "archived and
  no longer maintained! Forks are welcome." Last release v6.4.0.
- An Anki desktop add-on that reads the collection with SQL [`ankimorphs/recalc/anki_data_utils.py`].
- **How it defines "known"** [`ankimorphs/morpheme.py:59-77`, `recalc/caching.py:298-318`]:
  each card gets a strength value (new = 0, learning = 1, otherwise `ivl`). Each morph takes the
  **highest** value over all cards that contain it. Below `interval_for_known_morphs`
  (default 21, `config.json:54`) it's learning, at or above it's known. An option
  `use_stability_for_known_threshold` swaps in FSRS stability (`json_extract(cards.data, '$.s')`)
  for the interval. It never computes retrievability.
- **Morphemizers:** MeCab (from a companion add-on), jieba, spaCy, CAMeL Tools, a space splitter.
- **Exports:** `known_morphs-<datetime>.csv` (lemma, inflection) and imports a folder of them.
  Generators for readability reports and frequency-based study plans.
- **For Ankura:** reimplement the model (three states, highest strength across cards wins, lemma
  vs inflection, manual "known" tags). **Import AnkiMorphs' known-morphs CSV** to welcome its
  users, who now need a replacement. The code is AGPL and tied to Anki internals, so it's not
  reusable.

### MorphMan

- github.com/kaegi/MorphMan, last commit 2023-08-13. AnkiMorphs started as a fork and was then
  rewritten. `threshold_mature: 21` ("21 days is what Anki uses"). Historical interest only.

### asbplayer (best reference for the desktop pipeline)

- github.com/asbplayer/asbplayer (`killergerbah/asbplayer` redirects), AGPL-3.0, v1.22.0
  (2026-10). A subtitle player as a website and browser extension.
- **Colours subtitle words by Anki status** [`common/dictionary-db/dictionary-db-anki.ts`]: the
  user picks decks, word fields and sentence fields. Statuses come from `findCards` searches:
  `is:new` unknown, `is:learn` learning, then graduated / young / mature by `prop:s` (FSRS
  stability) with a fallback to `prop:ivl`. Mature defaults to 21 days and graduated to half of
  that. The source says "AnkiConnect doesn't expose Stability but we can retrieve it using search
  queries."
- Refreshes incrementally with `cardsModTime`/`notesModTime`, because `cardsInfo` is slow.
- Uses the Yomitan API for tokenizing.
- **For Ankura:** copy the approach (not the code): the status ladder, `prop:s` queries, and
  mod-time incremental refresh.

### VocabSieve

- github.com/FreeLanguageTools/vocabsieve, GPL-3.0, last commit 2025-08-15. Python/PyQt desktop
  sentence-mining tool.
- **Known-word model** [`vocabsieve/tools.py:459-467`, `config/tracking_tab.py`,
  `config/fieldmatcher.py`]:
  - The user edits two Anki searches (mature `prop:ivl>=14`, young `prop:ivl<14 is:review`).
  - Per note type, the user maps a **word field** and a **context field**.
  - Score = 8·seen + 15·lookups + 70·mature target + 30·mature context + 40·young target +
    20·young context. 100 or more counts as known.
  - Context fields are split on whitespace, so the context logic doesn't work for Japanese.
- **For Ankura:** the closest existing design to "Anki stats plus my own exposure and lookup
  stats". Borrow the per-note-type field map and the evidence-score idea. Change two things:
  lookups should **lower** confidence (VocabSieve adds to the score for them), and Japanese
  needs a real tokenizer.

### Yomitan

- github.com/yomidevs/yomitan, GPL-3.0, active (26.10.x). Successor to Yomichan.
- AnkiConnect for card creation, plus `cardsInfo` to show "already in Anki".
- Android: Firefox for Android (and Kiwi or Edge Canary).
- **Yomitan API** (github.com/yomidevs/yomitan-api): a native-messaging host serving HTTP on
  `127.0.0.1:19633` (`/termEntries`, `/kanjiEntries`, `/tokenize`, ...). Desktop only.
- **Dictionary format** (JSON term, kanji and meta banks in a zip) is the de facto standard:
  JMdict/JMnedict/KANJIDIC builds, frequency lists, pitch accent.
- **For Ankura:** import Yomitan-format dictionaries; write our own parser and deinflector,
  because Yomitan's code is GPL. Dictionary contents have their own licences (JMdict is
  CC BY-SA).

### Memento, KOReader + anki.koplugin

- **Memento** (GPL-2.0): Qt6 + mpv video player with Yomitan dictionaries and optional MeCab.
  AnkiConnect, card creation only.
- **anki.koplugin** (AGPL): creates notes from KOReader lookups over AnkiConnect on the LAN
  (`webBindAddress` 0.0.0.0). If Anki can't be reached, it **queues notes on the device and sends
  them later**.
- **For Ankura:** copy the offline queue for card creation from the phone.

### jidoujisho (archived)

- github.com/arianneorpilla/jidoujisho (`lrorpilla` still resolves), GPL-3.0. **Archived
  2026-10-10**; last commit 2025-10-20 (2.9.1). "Yuuna" is the folder name of the 2.x code, not a
  successor app.
- Flutter with Java platform channels. Japanese through `mecab_dart` + `ve_dart` with bundled
  IPADIC.
- AnkiDroid: `AddContentApi` plus a media insert and `<queries>` for package visibility. **Write
  only plus a duplicate check**; it never reads card stats.
- **For Ankura:** a working example of the AnkiDroid permission flow. Its archiving leaves room
  on Android.

### AnkiConnect

- The GitHub repo FooSoft/anki-connect was **archived on 2025-11-04** and says it moved to
  git.sr.ht/~foosoft/anki-connect. sr.ht wasn't reachable from here, so its current activity is
  unverified. AnkiWeb add-on 2055492159. GPL-3.0+.
- `cardsInfo` returns `interval`, `factor`, `type`, `queue`, `due`, `reps`, `lapses`, `left`,
  `mod`, fields, deck and model [`plugin/__init__.py:1464-1487`]. **It returns no FSRS stability or
  difficulty.** Related actions: `findCards`, `cardsModTime`, `getIntervals`, `getEaseFactors`,
  `cardReviews`, `getReviewsOfCards`, `exportPackage`.
- **For Ankura:** the primary desktop path. Get FSRS state through `prop:s`/`prop:r` searches;
  see `architecture-options.md` §2.

### AnkiconnectAndroid

- github.com/KamWithK/AnkiconnectAndroid, GPL-3.0, 1.15. An Android app that runs an
  AnkiConnect-compatible HTTP server on port 8765, backed by AnkiDroid's provider.
- Implements 15 actions [`routing/AnkiAPIRouting.java:45-88`]: `version`, `deckNames(AndIds)`,
  `modelNames(AndIds)`, `modelFieldNames`, `findNotes`, `guiBrowse`, `canAddNotes(...)`,
  `addNote`, `updateNoteFields`, `storeMediaFile`, `notesInfo`, `multi`. **No `findCards` or
  `cardsInfo`**, and unknown actions return a version reply rather than an error.
- **For Ankura:** not usable for stats. Ankura can query AnkiDroid directly (next entry).

### AnkiDroid ContentProvider

- Authority **`com.ichi2.anki.flashcards`** [`AnkiDroid/src/main/AndroidManifest.xml:633`,
  `api/.../FlashCardsContract.kt:110`]. Permission `com.ichi2.anki.permission.READ_WRITE_DATABASE`,
  `protectionLevel="dangerous"` (runtime prompt) [manifest lines 45-49]. Android 11+ also needs
  `<queries><package android:name="com.ichi2.anki"/></queries>` (AnkiDroid wiki, "AnkiDroid API").
- **Since v2.24.0 (2026-05-02)** there is a top-level `cards` URI whose `selection` is an **Anki
  search string** passed to `col.findCards()` (commit d20fd891a6, 2026-02). Card rows can return
  `interval`, `reps`, `lapses`, `type`, `queue`, `due`, `sm2_factor`, `left`, `custom_data`,
  `fsrs_stability`, `fsrs_difficulty`, `fsrs_desired_retention`, `fsrs_decay`,
  `last_review_time_secs` (commits f946f26893 and e6a8f1498c, 2026-04). `flags` arrived in 2.25.0.
  The `notes` URI also accepts an Anki search as `selection` and returns `flds` (all fields,
  0x1f-separated) and `mid`.
- The stable release now is 2.25.3 (2026-10-17). How many users have 2.24 or later is unknown.
- The `api` client library is LGPL and versioned 2.0.0. It isn't on Maven Central; JitPack only.
  Ankura can use the raw column names with `ContentResolver` and skip the library.
- **For Ankura:** this answers HANDOFF §4 #6. The phone can read Anki scheduling state directly.
  See `architecture-options.md` §2.

### Anki sync server

- Built into Anki (`anki --syncserver`, `python -m anki.syncserver`, or the `anki-sync-server`
  binary from `rslib/sync`). AGPL. Stores each user's collection at
  `SYNC_BASE/<user>/collection.anki2` [`rslib/src/sync/http_server/user.rs:89`].
- The manual warns that the protocol changes with client versions, that third-party servers are
  not recommended, and that a REST API is unlikely to be accepted (`anki-manual/src/sync-server.md`).
  The Python and Rust community servers are dead or archived.
- AnkiDroid supports it under Settings → Sync → Custom sync server.
- **For Ankura:** not a data API. Writing a sync-protocol client would be fragile. Don't.

### FSRS

- **fsrs-rs** (BSD-3, 6.6.2; Anki pins this version), **py-fsrs** (MIT), **ts-fsrs** (MIT). No
  official Kotlin library.
- Retrievability: `R(t, S) = (1 + F·t/S)^(−decay)` with `F = 0.9^(−1/decay) − 1`, so R = 0.9 when
  t = S. Decay is 0.5 for FSRS-4.5 and FSRS-5 and trainable for FSRS-6 (default 0.1542)
  [`fsrs-rs/src/inference.rs:38-41`, `inference_v6.rs:1-2`].
- Anki 26.09.3 stores a per-card `decay` and picks FSRS-5 or FSRS-6 decay from the parameter count
  [`rslib/src/scheduler/fsrs/memory_state.rs:37-43`]. fsrs-rs contains FSRS-7 code, but Anki
  26.09.3 doesn't use it, so the formula above holds for current Anki.
- Per-card FSRS state lives in the `cards.data` JSON: `s`, `d`, `dr`, `decay`, `lrt` (last review
  time, added in Anki 25.07.3) [`rslib/src/storage/card/data.rs`].

## 4. Closed-source tools worth learning from *(secondary sources)*

- **jpdb.io:** Japanese SRS + dictionary with decks per anime, novel and VN. Shows per-work
  **coverage** (share of word occurrences you know) next to unique-word known %. Users aim for
  about 95% before starting a work. Borrow: coverage by token count, not unique words, and "learn
  this work's top unknown words first".
- **LingQ:** blue (new) and yellow (saved, statuses 1-4) words, then Known and Ignored. An option
  marks remaining blue words as known on page turn. Borrow, as a setting: page-turn promotion
  grows the known count with no effort, but it also inflates it.
- **Migaku:** colours words and shows comprehension %. Anki status doesn't sync automatically; a
  third-party extension marks cards with an interval of 21 days or more as known through
  AnkiConnect and skips suspended cards. Borrow: exclude suspended cards and allow demotion.
- **Satori Reader:** graded serial stories with audio. Kanji, kana or furigana display, plus a
  **known-kanji list that hides furigana for words made only of known kanji**. Built-in SRS cards
  carry the story sentence. Borrow: furigana suppression driven by known kanji, which Ankura can
  derive from Anki.
- **Readlang:** a web reader. Click a word or drag across a phrase; the context sentence is saved;
  Anki gets a manual export. No AnkiConnect. AI text generation not found.
- **Language Reactor:** dual subtitles on Netflix and YouTube, with saved words highlighted
  everywhere and learning/known/skipped tags. Pro exports CSV for Anki. Borrow: known-word
  highlighting that carries across every source.

## 5. What Ankura should take from this

1. **Desktop Anki read path:** AnkiConnect `findCards` + `cardsInfo` + `cardsModTime`, FSRS via
   `prop:s`/`prop:r` searches (the asbplayer approach).
2. **Android Anki read path:** the AnkiDroid ContentProvider `cards` URI with an Anki search, on
   AnkiDroid 2.24 or later. Not AnkiconnectAndroid.
3. **Known-word model:** AnkiMorphs' three states with "highest strength across cards wins", FSRS
   retrievability where present, and VocabSieve's idea of combining Anki state with exposure and
   lookups.
4. **Configurable field mapping** per note type is standard practice (VocabSieve, asbplayer,
   Lute, AnkiMorphs). It suits a 20-year collection.
5. **Dictionaries:** import the Yomitan format. **Tokenizer:** every Japanese tool here uses
   MeCab-class tokenizers (MeCab/IPADIC, Sudachi). Pick one behind an interface.
6. **Imports that lower the switching cost:** AnkiMorphs known-morphs CSV, and `.apkg`/`.colpkg`.
7. **Reading UX:** vertical text, furigana toggles driven by known kanji, character-count stats
   and goals, coverage before reading.
8. **Code reuse:** only from MIT/BSD/Apache/public-domain sources (FSRS libs, Lute, Lukaisu,
   Jiten, LWT, ReadBridge). Everything else is ideas only.
