# Claims check: prior AI report

Checks every claim in HANDOFF.md §4 against the real repos and docs, then the other factual claims in
`prior-ai-report.md`. Researched 2026-10-26. Verdicts:

- **Confirmed:** matches the source.
- **Wrong:** contradicted by the source; the correction is given.
- **Partly:** some of it is right, some wrong or missing.
- **Unverifiable:** couldn't be checked from here; says what would settle it.

Paths in brackets are files inside the named repo. Supporting detail is in `landscape.md`.

## HANDOFF §4 claims

| # | Claim | Verdict |
|---|---|---|
| 1 | Lute v3 at `jzohrab/lute-v3`; Python/Flask/SQLite; MeCab, Jieba, spaCy | **Partly** |
| 2 | LWT fork `HugoFara/lwt` has a Kotlin Android client "Lukaisu" | **Partly** (exists, not Kotlin) |
| 3 | AnkiMorphs tracks known morphs by interval, scores n+1 | **Confirmed**, but now archived |
| 4 | Jidoujisho is a Flutter Android app using AnkiDroid's API | **Confirmed**, but archived, write only |
| 5 | read-bridge does LLM-driven n+1 reading | **Partly** |
| 6 | AnkiDroid provider URI `content://com.ichi2.anki.provider/cards` | **Wrong** authority; stats **are** exposed |
| 7 | AnkiConnect `cardsInfo` returns FSRS stability and difficulty | **Wrong** |
| 8 | Known = interval ≥ 21d or stability ≥ 30 | **Convention, not a fact**; use retrievability |
| 9 | Tauri v2 targets Android with a Kotlin plugin bridge; Lindera runs there | **Confirmed** in principle, not tested on a device |
| 10 | Never read `collection.anki2` while Anki runs | **Confirmed**, and backups are a safe source |
| 11 | Android 14+ kills long background work | **Generic**, low relevance |

### 1. Lute v3: Partly

- **Location:** moved to github.com/LuteOrg/lute-v3. `jzohrab/lute-v3` redirects there (same HEAD
  SHA `933d084`). MIT, tag 3.10.3 (2026-07-07), HEAD 2026-09-04.
- **Stack:** Python, Flask, Flask-SQLAlchemy, SQLite. Confirmed (`pyproject.toml`).
- **Parsers:** MeCab (`lute/parse/mecab_parser.py`, via natto-py) and jieba (`plugins/lute-mandarin`)
  confirmed. **spaCy is wrong**: no reference anywhere in the repo. There are also Thai
  (pythainlp) and Khmer plugins, and a character parser.
- **Anki integration:** the report's own text says "Anki export via CSV/TSV or AnkiConnect". The
  AnkiConnect part is right: since 3.10.0 (2025-02-23, CHANGELOG #579) there is a configurable
  export with deck, note type, field mapping and criteria (`lute/ankiexport/`). It's one way;
  Lute never reads Anki stats. The manual (`src/usage/ankiexport/index.md`) says it doesn't work
  from a mobile device.
- **Mobile:** none. The manual offers "connect to a running instance" from a phone browser, or
  Termux (`src/install/android-using-termux.md`). No `LuteOrg/lute-android` repo.
- **Statuses:** 1-5, 98 Ignored, 99 Well known (`lute/models/term.py:315-317`); multi-word terms
  and reading stats (`wordsread` table) confirmed.

### 2. LWT and "Lukaisu": Partly

- `HugoFara/lwt` was renamed to **HugoFara/learning-with-texts** (the old URL redirects).
  Unlicense, very active (3.7.0, 2026-09-04). PHP 8.2+, MySQL/MariaDB. Confirmed.
- **Lukaisu exists**, so the setup agent's suspicion that it was invented is wrong. It is at
  github.com/lukaisu/lukaisu (MIT, HEAD 2026-07-04). But it is **not Kotlin**: it's Capacitor 8 +
  Svelte 5 + TypeScript, with Dexie and ts-fsrs. The only native code is a stub `MainActivity.java`.
  Local-first, with an optional LWT-compatible server for CJK segmentation.
- The report's "No SRS intelligence, no Anki sync" for LWT is **wrong**: LWT stores FSRS-6 state per
  term (CHANGELOG #238) and imports and exports `.apkg`, replaying Anki's revlog through its own
  scheduler (CHANGELOG #264). "No AI" was not checked in depth.

### 3. AnkiMorphs: Confirmed, with corrections

- github.com/mortii/anki-morphs, AGPL-3.0. **Archived by the owner on 2026-10-23.**
- **"Known" definition** (`ankimorphs/morpheme.py:59-77`, `recalc/caching.py:298-318`): card
  strength = 0 for new, 1 for learning, otherwise `ivl`. A morph takes the **maximum** over all its
  cards. Below `interval_for_known_morphs` (default **21**, `config.json:54`) it's learning, at or
  above it's known. An option swaps in FSRS stability (`json_extract(cards.data,'$.s')`). No
  retrievability.
- **n+1:** confirmed. It sorts and tags cards by the number of unknown morphs.
- **Parsers:** the report says spaCy and CAMeL. Incomplete: Japanese uses **MeCab** through a
  companion add-on, and jieba is also supported.
- **Reusable?** As ideas, yes. As code, no: it's AGPL (Ankura is MIT) and tied to `aqt.mw` and
  Anki's SQL schema. It exports a `known_morphs-*.csv` that Ankura can import.

### 4. Jidoujisho: Confirmed, with corrections

- Canonical repo is github.com/arianneorpilla/jidoujisho (`lrorpilla` still resolves). GPL-3.0.
  **Archived on 2026-10-10.** Last commit 2025-10-20 (2.9.1).
- Flutter with Java platform channels: confirmed. It uses AnkiDroid's `AddContentApi` and a media
  insert through `ContentResolver` (`yuuna/android/.../AnkiDroidHelper.java`). It only **adds
  notes and checks duplicates**; it never reads card stats.
- **Successor:** none found. "Yuuna" is the folder of the 2.x code in the same repo.
- Tokenizer: `mecab_dart` + `ve_dart`, IPADIC bundled as assets (`yuuna/pubspec.yaml`).
- The table's "Lookup history" and "AI: Minimal" weren't checked.

### 5. read-bridge: Partly

- Exists: github.com/WindChimeEcho/read-bridge, MIT, HEAD 2026-09-18, v0.1.11.
- It's **Next.js + Tauri 2**, not Electron. Web and desktop only.
- It describes itself as an n+1 reading assistant, but what it does is LLM explanation of the
  user's own books sentence by sentence, at a self-chosen level. It **does not generate n+1
  texts**, track known words, or talk to Anki (no "anki" in the source). The table's "Manual
  export" Anki sync is wrong: there is none.

### 6. AnkiDroid ContentProvider: Wrong URI; the stats are there

This was the critical item.

- **Authority:** `com.ichi2.anki.flashcards`, as the setup agent thought. The manifest declares
  `android:authorities="${applicationId}.flashcards"` (`AnkiDroid/src/main/AndroidManifest.xml:633`)
  and `FlashCardsContract.AUTHORITY = "${ANKI_PACKAGE_NAME}.flashcards"`
  (`api/src/main/java/com/ichi2/anki/FlashCardsContract.kt:110`). `com.ichi2.anki.provider` is wrong.
- **Card search URI:** `content://com.ichi2.anki.flashcards/cards`. Its `selection` is an Anki
  search string passed to `col.findCards()` (`AnkiDroid/.../provider/CardContentProvider.kt`,
  `CARDS ->` branch). Added in commit d20fd891a6 (2026-02-04), first released in **v2.24.0**.
  Single cards are at `cards/<id>`; per-note cards are at `notes/<id>/cards`.
- **Scheduling columns** (`FlashCardsContract.Card`, commits f946f26893 and e6a8f1498c, April 2026,
  also first released in v2.24.0 on 2026-05-02):
  `interval`, `reps`, `lapses`, `type`, `queue`, `due`, `original_due`, `sm2_factor`, `left`,
  `custom_data`, `original_position`, **`fsrs_stability`**, **`fsrs_difficulty`**,
  **`fsrs_desired_retention`**, **`fsrs_decay`**, **`last_review_time_secs`**. `flags` came in
  v2.25.0. FSRS columns are `null` for cards without memory state.
- **Not exposed:** the review log (revlog) and the collection's day cutoff. A card row is enough
  to compute FSRS retrievability, but not to convert a review card's raw `due` (a day number) to a
  date.
- **Permission:** `com.ichi2.anki.permission.READ_WRITE_DATABASE`, `protectionLevel="dangerous"`,
  so a runtime prompt (manifest lines 45-49). Android 11+ also needs a `<queries>` entry for
  `com.ichi2.anki` (AnkiDroid wiki "AnkiDroid API"). Debug and parallel builds use a different
  package name, so the authority and permission change with them.
- Current stable is v2.25.3 (2026-10-17). AnkiDroid before 2.24 has notes and per-note cards, but
  no scheduling columns.
- **Consequence:** the Android plan doesn't have to change. The phone can read Anki stats
  directly. A fallback is still needed for older AnkiDroid versions and for users who deny the
  permission (see `architecture-options.md` §2).

### 7. AnkiConnect `cardsInfo` and FSRS: Wrong

- `cardsInfo` returns `cardId`, `fields`, `fieldOrder`, `question`, `answer`, `modelName`, `ord`,
  `deckName`, `css`, `factor`, `interval`, `note`, `type`, `queue`, `due`, `reps`, `lapses`,
  `left`, `mod` (`plugin/__init__.py:1464-1487` in FooSoft/anki-connect). **No stability, no
  difficulty**, and no `memory_state` anywhere in the add-on. asbplayer's source has the same
  finding: "AnkiConnect doesn't expose Stability".
- **Workaround:** Anki search supports `prop:s`, `prop:d` and `prop:r` (stability, difficulty,
  retrievability) from Anki 23.10 with FSRS enabled (`anki-manual/src/searching.md:340-349`), so
  `findCards` with range queries can bucket cards by FSRS state. Review history is available via
  `getReviewsOfCards`.
- **Status:** the GitHub repo was archived on 2025-11-04 and points to
  git.sr.ht/~foosoft/anki-connect. That host wasn't reachable here. Whether a newer version added
  FSRS fields is **unverifiable** from here. It's worth checking from a normal network.

### 8. "Known = interval ≥ 21d or stability ≥ 30": a convention

- 21 days is Anki's own "mature" threshold, carried through MorphMan (`threshold_mature: 21`),
  AnkiMorphs and asbplayer. VocabSieve uses 14. The "stability ≥ 30" figure has no source and is
  inconsistent with the interval rule. AnkiMorphs compares stability against the same 21.
- **Better signal: FSRS retrievability now.** `R(t, S) = (1 + F·t/S)^(−decay)`,
  `F = 0.9^(−1/decay) − 1`, where t is days since the last review
  (`fsrs-rs/src/inference.rs:38-41`). Anki stores per-card `decay` (FSRS-5: 0.5, FSRS-6: trained,
  default 0.1542) and `lrt` (last review time, since Anki 25.07.3) in `cards.data`. Anki 26.09.3
  doesn't use FSRS-7 (`rslib/src/scheduler/fsrs/memory_state.rs:37-43`), so this formula is
  exact for current Anki.
- R answers "how likely is the user to recall this today", which is what the reader needs. It
  falls for an overdue card even if its interval is long. A mix of stability (how durable) and R
  (how fresh) is the proposal in `architecture-options.md` §3. For SM-2 cards, fall back to the
  interval.

### 9. Tauri v2 on Android: Confirmed in principle

- Tauri 2.12.5 (2026-10-21) builds Android apps. Mobile plugins are Kotlin classes (`@TauriPlugin`,
  `@Command`) with `activity` access, so `activity.contentResolver.query(...)` is plain Kotlin
  (`tauri/crates/tauri/mobile/android/.../app/tauri/plugin/`).
- `Permission(strings = [...])` takes any string and uses the standard runtime request, so
  `com.ichi2.anki.permission.READ_WRITE_DATABASE` should work. **Not tested on a device.**
- **Maturity:** 91 open Android issues, including UI-thread deadlocks (#16169), lost Activity
  callbacks that hang an invoke (#16122, which matters for permission requests), keyboard and
  viewport bugs (#10631), and the back button exiting the app (#14406). Tauri 3 is in alpha
  (3.0.0-alpha.5), so a migration is likely later.
- **Real-world proof:** Readest (an e-book reader) ships Tauri v2 to Android and iOS.
- **Lindera:** pure Rust (6.2.0), so it should cross-compile to Android, but nobody here has built
  it for Android. Dictionaries are the size problem: IPADIC is 47.5 MB unpacked (10.5 MB zipped),
  UniDic 199 MB (46 MB zipped). Embedding IPADIC into the native library puts about 50 MB into the
  APK, because modern APKs store native libraries uncompressed. Shipping it as a compressed asset
  or downloading it on first run is smaller. **APK size is unverified until a test build.**
- No existing Tauri plugin calls a third-party ContentProvider. Tauri's own `dialog` and `fs`
  plugins use `contentResolver`, which is the nearest example.

### 10. Don't read `collection.anki2` while Anki runs: Confirmed

- The manual: "You should never copy or move your collection while Anki is open"
  (`anki-manual/src/files.md:65-66`).
- The reason is in the code: Anki opens the database with `locking_mode=exclusive` and
  `journal_mode=wal` (`rslib/src/storage/sqlite.rs:64-68`). Other processes can't open it while
  Anki has it, and a file copy can miss changes still in the WAL file.
- **Reading a backup is safe and useful.** Anki writes automatic backups every 30 minutes by default
  (`anki-manual/src/backups.md`) as `backup-YYYY-MM-DD-HH.MM.SS.colpkg`
  (`rslib/src/collection/backup.rs:23`). A `.colpkg` is a zip with `collection.anki21b`, a
  zstd-compressed SQLite file (`rslib/src/import_export/package/meta.rs:29`). Backups are written
  once and then not touched, so Ankura can read the newest one without risk. It includes the full
  `cards.data` JSON with FSRS state. Its age is up to the backup interval. A manual `.colpkg`
  export works the same way, including on AnkiDroid.
- Reading a copy of the live file that Anki didn't make is unsafe. Don't.

### 11. Android 14+ kills long background work: Generic

- True as a general statement about background execution limits. It matters only if Ankura runs
  heavy background work on the phone. The design reads AnkiDroid on demand (when the app is open)
  and runs LLM calls in the foreground, so it should never need long background jobs. Not
  researched further.

## Other claims in the prior report

| Claim | Verdict |
|---|---|
| Lute "No awareness of Anki intervals or stability" | **Confirmed**. Export only. |
| Lute "Tracks reading sessions" | **Confirmed**: `wordsread` table (`lute/models/book.py:261`). |
| LWT "Docker support in community branches" | **Confirmed**: Docker files in the main fork. |
| AnkiMorphs table: "Direct DB" | **Confirmed**: reads through `mw.col.db` inside Anki. |
| Jidoujisho table: "AnkiDroid ContentProvider" | **Partly**: it uses `AddContentApi` (which wraps the provider) and writes only. |
| Yomitan: "Browser extension, AnkiConnect export, deinflection" | **Confirmed**. It also has a desktop HTTP API (yomitan-api, port 19633). |
| read-bridge: "Web/Electron", "Manual export" to Anki | **Wrong**: Tauri 2, no Anki link. |
| Flutter would need MeCab as native `.so` | **Partly**: jidoujisho did exactly that with `mecab_dart`. Alternatives: a Dart Kuromoji port, or a Rust tokenizer via `flutter_rust_bridge`. |
| AnkiConnect at `http://127.0.0.1:8765` | **Confirmed**. |
| Permission `com.ichi2.anki.permission.READ_WRITE_DATABASE` | **Confirmed**: dangerous level, runtime prompt. |
| Sync via CRDT, ElectricSQL, or self-hosted | **Outdated**: ElectricSQL pivoted to a read-path Postgres sync engine that needs a server; cr-sqlite has had no release since January 2024. See `architecture-options.md` §5. |
| "Yomitan cards often store inflected forms", so join on lemma | **Plausible, not checked**. Lemma normalization is in the design either way. |
| Piper or Edge-TTS for audio | **Not checked**. Out of scope for phase 1. |
| CI ≥ 0.95 as the target | **A convention**: about 95% coverage is the target jpdb users cite *(secondary)*. Keep it configurable. |

## Corrections in one place

1. AnkiDroid authority is `com.ichi2.anki.flashcards`. Since 2.24.0 its `cards` URI takes an Anki
   search and returns interval, lapses and FSRS state.
2. AnkiConnect `cardsInfo` has no FSRS fields; use `prop:s`/`prop:r` searches or backups.
3. Lute is at LuteOrg, uses no spaCy, and has had AnkiConnect export since 3.10.
4. Lukaisu exists but is Capacitor/Svelte, not Kotlin. LWT does have FSRS and `.apkg` round trips.
5. read-bridge is Tauri, not Electron, and generates no texts.
6. AnkiMorphs and jidoujisho were both archived in October 2026.
7. Reading Anki's newest automatic backup is a safe, offline source of full FSRS state.
