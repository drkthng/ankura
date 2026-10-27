# Handoff to the next session

Updated 2026-10-27, at the end of the second working session (context is cleared after it).
Read this first, then `CLAUDE.md`, `docs/ROADMAP.md` and the ADRs it points to.
`docs/HANDOFF.md` is the original phase-1 brief; its research tasks are done.

## 1. Where things stand

- **M1 and M2 done and in daily use** on the owner's real collection (Windows), with their
  Anthropic API key (Max plan API credits, ADR 0011).
- **M3 built** (ADRs 0012-0014, ROADMAP): Anki cards from reading, phrase selection and phrase
  matching, "Simplify this sentence", stats and stealth leeches, kanji (pop-up, RTK mnemonics,
  consistent primitive names), correctable furigana, interest profile, level check.
- **Confirmed by the owner on their collection:** the API key works; card fields pick the meaning
  that fits the sentence; kanji setup: both note types mapped (Kanji / Word / Mnemonic), sync read
  2,508 kanji; "all looks good".
- **Not yet tried by the owner** (tested with fakes only): new mnemonic in the reader, primitive
  analysis on the Kanji screen (2,508 stories ≈ 63 AI requests), level check, stats over days,
  interests, reading fixes.
- Last commit `bee00de` on `main`. Latest Windows installer given to the owner (kanji setup fix):
  https://github.com/drkthng/ankura/actions/runs/40498469483/artifacts/12731158630
- Tests: 76 core tests (`cargo test -p ankura-core`), type-check (`npm run check`), WebDriver e2e
  (`tools/dev/e2e_smoke.py`) covering every screen. All passing.
- **User docs:** `README.md` links one guide per feature in `docs/guide/`; update the matching
  guide with every user-facing change (CLAUDE.md says so too). Build docs: `docs/DEVELOPING.md`.

## 2. The owner and how they work

- **Uses the app, doesn't build it.** Windows 11 (German UI). Installs the `.exe` from the CI
  artifact. They tried installing the C++ Build Tools and it failed (exit 1602); don't push them
  toward a local toolchain. After every user-facing change: push → wait for the **Build** workflow
  → give them the direct artifact link (see §4).
- **Working rules:** push straight to `main`, no PRs unless asked. Questions via `AskUserQuestion`.
  Plain English, short, non-technical when talking to them. They respond well to screenshots
  being requested when something looks off.
- **Their Anki collection** (what the code was tuned on):
  - One mapped note type, **"Japanisch"**, 7,646 notes, fields **"Original Sentence"**,
    **"Reading"**, plus a definition field.
  - Sentence cards: the word is **bold/italic** in the sentence (mapping mode "marked"). Some
    sentences have nothing marked; then the reading field often names the word.
  - The reading field's **first line is the word's reading**; the full sentence reading follows.
  - **6,030 of 7,646 cards are suspended on purpose.** They lost context during a 6-month break and
    decided to start fresh; treat suspended as "doesn't count" (the default). Only ~1,616 notes are
    studied.
  - **FSRS is off** in their Anki, so recall is estimated from interval + overdue days.
  - **Kanji** and **Kanji reverse** are mapped as kanji notes (fields Kanji / Word = keyword /
    Mnemonic); 2,508 kanji. They learn kanji with RTK; the mnemonic is hidden on both cards.
    TopicByTag and English are unmapped.
  - Last sync (2026-10-27): 1,622 notes, 1,510 words, 108 phrases, 4 without a word; 1,578 known,
    119 fading, 55 learning; 845 cards 30+ days overdue.
- They mentioned not having practised for 6 months; fading words matter to them.
- They ask for features mid-task and like them recorded and built in order; they pick between
  options well when asked with `AskUserQuestion` (recap first).

## 3. Code map

| Path | What |
|---|---|
| `crates/ankura-core/src/tokenize.rs` | `Tokenizer` trait, Sudachi impl (mode C + mode A parts), token kinds, auxiliary-after-て rule, preferred readings (私→わたし) |
| `counters.rs` | Number + counter readings (三匹 さんびき, 一本 いっぽん; 一日 いちにち vs ついたち), applied after tokenizing |
| `furigana.rs`, `kana.rs` | Ruby segments over kanji only; kana helpers |
| `normalize.rs` | Anki field cleaning (HTML, ruby, `漢字[かんじ]`, cloze, sound, lists, brackets); `marked_text` (bold/italic/underline) |
| `mapping.rs` | Field → word keys (light verbs, phrases + their words, reading plausibility, unmarked-sentence fallback, word picked by reading), preview, `suggest_mapping` (content-based), `sync_anki` (AnkiConnect, FSRS buckets via `prop:s`/`prop:r`, overdue buckets via `prop:due` without FSRS, per-note-type report, `Scope` excludes suspended) |
| `kanji.rs` | Kanji note mappings (+ `suggest_mapping`), kanji table filled by sync, `card` (pop-up), `new_story` (RTK mnemonic), `save_story` / `undo_story` (mnemonic field only, `kanji_edits` backup, tag `ankura-edited`) |
| `primitives.rs` | Consistent primitive names: `analyse` (batches of 40, progress, stop, resumable; `kanji_primitives`, `kanji_analysed`), `list` (clashes first), `approve` (`primitive_names`, also used by `kanji::new_story`), `candidates`, `rewrite`, `save_rewrite` |
| `interests.rs` | Interest profile (setting `interests`: weighted topics, avoid list), `pick` (by weight), `suggest` (AI, from pasted texts and asked-for topics) |
| `proficiency.rs` | Level check (ADR 0014): bundled JLPT lists (`data/`, tanos CC BY), vocab yes/no with pseudowords, kanji readings with rule distractors, AI grammar/reading checked by a solver call, latent-level scoring (`p_right` guessing model), `tests`/`test_items` |
| `stats.rs` | Reading stats: `reading_days` (chars per local day, logged when a paragraph counts as read), streaks, stealth leeches, most looked up |
| `knowledge.rs` | Card facts → word state (unknown/new/learning/fading/known), strongest card wins |
| `store.rs` | SQLite, schema v4 with migrations from v1; texts (+`extra` JSON), mappings (+`target_mode`), knowledge (+`display`), overrides, events (+`display`, device ID), dictionary tables; `study_words`, `known_lemmas` |
| `dict.rs` | Yomitan zip import (plain and structured-content glossaries), lookup by spelling or reading |
| `reader.rs` | Text → paragraphs of tokens with furigana, state (mark > anki > basic word > parts), coverage; known phrases merged into one token (`phrase_form`: 気になります → 気になる, also used by sync); `phrase` (a selection as one token) |
| `llm.rs` | Anthropic Messages (raw HTTPS, `anthropic-version: 2026-10-21`, `output_config` format+effort, `fallbacks: "default"`) and OpenRouter |
| `generate.rs` | `prepare` / `check_draft` / `feedback` / `save`, shared by the automatic loop (`generate_text`: retries, best attempt saved) and copy & paste (`chat_prompt`, `parse_answer` lenient, `submit_answer`); `simplify_sentence` |
| `anki.rs` | `AnkiSource` trait, AnkiConnect client with an action allow-list (add-only writes: `canAddNotesWithErrorDetail`, `addNote`), in-process fake for tests |
| `cards.rs` | Card drafts (ADR 0012): `new_sentence` (i+1, checked, retried), `fill_fields` (AI copies the three newest notes of the chosen type), `drafts_for_text`, `add_to_anki` (tag `ankura`) |
| `src-tauri/src/lib.rs` | App state, all Tauri commands (blocking work via `spawn_blocking`) |
| `src-tauri/src/downloads.rs` | First-run downloads: SudachiDict (PyPI JSON API), JMdict (yomidevs GitHub release); OS certificate store |
| `src-tauri/src/secrets.rs` | API keys in the OS keychain (`ANKURA_API_KEY` env overrides for dev) |
| `src/views/*.svelte` | Library, Reader (phrase selection, cards for targets), Write ("Write for me": API or copy & paste, "Make cards"), Cards (card list), Stats, Kanji (primitive names), Test (level check), AnkiSetup (incl. kanji note mapping), Settings |
| `src/components/` | `WordPopover`, `KanjiPanel`, `CoverageBar` |
| `tools/dev/` | `fake_ankiconnect.py` (incl. a "Satzkarten" note type like the owner's), `fake_llm.py`, `jamdict_to_yomitan.py`, `e2e_smoke.py` |
| `.github/workflows/build.yml` | Check + tests, then installers for Windows/macOS/Linux as artifacts |
| `spikes/s3-tokenizers/` | Tokenizer comparison (gold set, report) behind ADR 0008 |

Decisions: ADRs 0001-0012 in `docs/adr/`. Most relevant now: 0012 (Anki cards, phrases), 0011 (AI without API costs; Max plan API credits), 0008 (Sudachi), 0009 (desktop +
Japanese first), 0010 (generated texts, default model `claude-opus-5-5`, effort medium).

## 4. Working in a cloud session (what tripped this session up)

- **GitHub access:** attach the repo with the `add_repo` tool (`drkthng/ankura`, access `push`).
  `gh` GraphQL is blocked: use REST (`gh api ...`). Draft PR: `gh api repos/drkthng/ankura/pulls -X
  POST -F draft=true ...`. CI status:
  `gh api repos/drkthng/ankura/actions/runs?head_sha=$(git rev-parse HEAD)`. Artifact link for the
  owner: `https://github.com/drkthng/ankura/actions/runs/<run_id>/artifacts/<artifact_id>` (ids
  from `gh api repos/drkthng/ankura/actions/runs/<run_id>/artifacts`).
- **E2E flakiness:** WebKitWebDriver sometimes drops the session ("Remote end closed connection"),
  usually right after the copy & paste flow writes to the clipboard (WebKitGTK under Xvfb?
  unverified; Linux only, the Windows app uses WebView2); kill leftovers (`pkill -x Xvfb ankura tauri-driver WebKitWebDriver` one by
  one) and rerun. The fakes send `Connection: close` (ureq reused closed connections). The
  driver's and app's output goes to `<out>/tauri-driver.log` (a "Connection refused" backtrace
  at startup is harmless: the driver retries).
- **Linux build prerequisites:**
  `apt-get install -y libwebkit2gtk-4.1-dev build-essential libssl-dev libxdo-dev
  libayatana-appindicator3-dev librsvg2-dev fonts-noto-cjk webkit2gtk-driver xdotool imagemagick`,
  plus `cargo install tauri-driver --locked` for the e2e test.
- **Dictionaries for development:** GitHub release downloads and edrdg.org are unreachable from the
  container; PyPI works.
  - SudachiDict: download the `SudachiDict-core` wheel from PyPI and unzip
    `sudachidict_core/resources/system.dic`. Set `ANKURA_SUDACHI_DIC` to it for tests and the app.
    Without it, the tokenizer tests skip silently.
  - JMdict: `pip download --no-deps --no-binary :all: jamdict-data==1.5`, unpack, `xz -d`, then
    `python3 tools/dev/jamdict_to_yomitan.py jamdict.db JMdict_dev.zip`.
- **E2E:** `npx tauri build --debug --no-bundle`, then
  `python3 tools/dev/e2e_smoke.py --sudachi <system.dic> --dict <JMdict zip> --out <dir>`. It starts
  Xvfb, fake AnkiConnect (8765), fake LLM (8766) and tauri-driver, and saves screenshots (read them).
  Element screenshots come out black; scroll with JS and take a page screenshot instead.
- **Don't `pkill -f`** with patterns that also match your own shell command; use `pkill -x Xvfb`,
  `pkill -x ankura`, `pkill -x tauri-driver`, `pkill -x WebKitWebDriver`.
- Python `urllib` goes through the session proxy for localhost unless you pass
  `ProxyHandler({})` (the e2e script does).
- The `claude-api` skill must be loaded before touching `llm.rs` or model choices.

## 5. Next steps, in order

1. **Support the owner's first real use** of what's untested (§1). Likely issues: the AI's story
   language/formatting vs theirs (`kanji::new_story`), quality and cost of the primitive analysis
   and the clash list (`primitives.rs`), level check items from the real model (the solver check
   may drop many; `grammar_items` errors if none survive).
2. **Next feature (owner's order):** JLPT practice tests per level (research
   `docs/research/proficiency-tests.md` §Recommendations v1.5), then listening (TTS), Italian
   (CILS) and Spanish (DELE/SIELE). Ask the owner before starting; they may have new requests.
3. **Known gaps:** incremental Anki sync (`cardsModTime`); context-dependent readings (十分,
   人気のない, この方) are guessed (user can fix them; `reading_fixes`); Sudachi splits some words
   oddly (一時的 → 一|時|的); the level check's `p_right` rates are assumptions (stored answers can
   calibrate them); JMdict download (`downloads.rs`) never run by the owner (fallback: import a
   Yomitan dictionary).
4. **Later:** code signing for installers (Windows SmartScreen warns); Android (spikes S1/S2,
   ADR 0004) and sync (ADR 0005); Italian/Spanish lemmatizer (ADR 0001); sudachi.rs needs a
   one-line Android patch (ADR 0008); Tauri 3 is in alpha. Claude Code CLI and free OpenRouter
   models for AI (ADR 0011) are on hold.
