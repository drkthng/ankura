# Handoff to the next session

Written 2026-10-26, at the end of the first working session (context was cleared after it).
Read this first, then `CLAUDE.md`, `docs/ROADMAP.md` and the ADRs it points to.
`docs/HANDOFF.md` is the original phase-1 brief; its research tasks are done.

## 1. Where things stand

- **M1 (read with your Anki knowledge): done, in daily use by the owner** on their real collection
  (Windows). Paste a text, read it coloured by Anki state, tap words for meanings, telemetry.
- **M2 (AI-written texts): built and tested with a fake provider only.** "Write for me" screen,
  Anthropic + OpenRouter, coverage check with retries, example sentences. **Not yet run against
  the real Claude API** (needs the owner's key). The owner's first real run is the first real
  test: expect to fix something there.
- Commit `bdb33c7` on `main` is the M2 commit; its CI run is
  https://github.com/drkthng/ankura/actions/runs/40380564839. Check, tests, Windows and macOS
  installers passed (Linux was still building at hand-off). Windows installer given to the owner:
  https://github.com/drkthng/ankura/actions/runs/40380564839/artifacts/12678018674
- Tests (at the M1/M2 hand-off; now 52): 42 core tests (`cargo test -p ankura-core`), type-check (`npm run check`), WebDriver e2e
  (`tools/dev/e2e_smoke.py`). All passing at hand-off.

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
  - Other note types (Kanji, Kanji reverse, TopicByTag, English) are unmapped.
  - Last sync: 1,474 words, 138 phrases, 4 notes without a word; ~1,640 known words.
- They mentioned not having practised for 6 months; fading words matter to them.

## 3. Code map

| Path | What |
|---|---|
| `crates/ankura-core/src/tokenize.rs` | `Tokenizer` trait, Sudachi impl (mode C + mode A parts), token kinds, auxiliary-after-て rule, preferred readings (私→わたし) |
| `furigana.rs`, `kana.rs` | Ruby segments over kanji only; kana helpers |
| `normalize.rs` | Anki field cleaning (HTML, ruby, `漢字[かんじ]`, cloze, sound, lists, brackets); `marked_text` (bold/italic/underline) |
| `mapping.rs` | Field → word keys (light verbs, phrases + their words, reading plausibility, unmarked-sentence fallback, word picked by reading), preview, `suggest_mapping` (content-based), `sync_anki` (AnkiConnect, FSRS buckets via `prop:s`/`prop:r`, overdue buckets via `prop:due` without FSRS, per-note-type report, `Scope` excludes suspended) |
| `knowledge.rs` | Card facts → word state (unknown/new/learning/fading/known), strongest card wins |
| `store.rs` | SQLite, schema v4 with migrations from v1; texts (+`extra` JSON), mappings (+`target_mode`), knowledge (+`display`), overrides, events (+`display`, device ID), dictionary tables; `study_words`, `known_lemmas` |
| `dict.rs` | Yomitan zip import (plain and structured-content glossaries), lookup by spelling or reading |
| `reader.rs` | Text → paragraphs of tokens with furigana, state (mark > anki > basic word > parts), coverage; `phrase` (a selection as one token: 気になります → 気になる) |
| `llm.rs` | Anthropic Messages (raw HTTPS, `anthropic-version: 2026-10-21`, `output_config` format+effort, `fallbacks: "default"`) and OpenRouter |
| `generate.rs` | `prepare` / `check_draft` / `feedback` / `save`, shared by the automatic loop (`generate_text`: retries, best attempt saved) and copy & paste (`chat_prompt`, `parse_answer` lenient, `submit_answer`) |
| `anki.rs` | `AnkiSource` trait, AnkiConnect client with an action allow-list (add-only writes: `canAddNotesWithErrorDetail`, `addNote`), in-process fake for tests |
| `cards.rs` | Card drafts (ADR 0012): `new_sentence` (i+1, checked, retried), `fill_fields` (AI copies the three newest notes of the chosen type), `drafts_for_text`, `add_to_anki` (tag `ankura`) |
| `src-tauri/src/lib.rs` | App state, all Tauri commands (blocking work via `spawn_blocking`) |
| `src-tauri/src/downloads.rs` | First-run downloads: SudachiDict (PyPI JSON API), JMdict (yomidevs GitHub release); OS certificate store |
| `src-tauri/src/secrets.rs` | API keys in the OS keychain (`ANKURA_API_KEY` env overrides for dev) |
| `src/views/*.svelte` | Library, Reader (phrase selection, cards for targets), Write ("Write for me": API or copy & paste, "Make cards"), Cards (card list), AnkiSetup, Settings |
| `src/components/` | `WordPopover`, `CoverageBar` |
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
- **E2E flakiness:** WebKitWebDriver sometimes drops the session ("Remote end closed connection")
  at a random step; kill leftovers (`pkill -x Xvfb ankura tauri-driver WebKitWebDriver` one by
  one) and rerun. The fakes send `Connection: close` (ureq reused closed connections).
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

0. **Anki cards (ADR 0012) and phrase selection: built, tested with fakes only.** The owner's
   first real use is the real test: check that the fields come out like their "Japanisch"
   notes (bold word in "Original Sentence", reading field = word reading, `<br>`, sentence
   reading), and that `addNote` goes into the right deck.
1. **First real run.** Done: the API key works (2026-10-26). The owner's Max plan includes $200/month of API credits (ADR 0011,
   Update); they claimed them and have an Anthropic API key, which goes into Settings. Copy &
   paste through claude.ai is built as the fallback (setting `ai_mode`; `chat_prompt`,
   `chat_submit` in `generate.rs`). Claude Code CLI and free OpenRouter models are on hold. Help
   with the first real run.
   Likely issues: error messages from the real API, prompt quality, texts landing under 95%
   coverage for their small known-word set (~1,600 words); tune the prompt or the threshold
   (`TARGET_COVERAGE` in `generate.rs`).
2. **JMdict download** (`downloads.rs`, GitHub release URL) has never been run; if it fails for the
   owner, the fallback is "Import a Yomitan dictionary…".
3. **Remaining M2:** "Simplify this sentence" (ROADMAP).
4. **Known gaps:** phrase cards aren't matched in the reader (their words count instead);
   incremental Anki sync (`cardsModTime`); context-dependent readings (十分, 人気のない, この方)
   and counter sound changes (三匹) are wrong in furigana; Sudachi splits some words oddly
   (一時的 → 一|時|的).
5. **M3** (ROADMAP): interest profile, stats and "stealth leeches", kanji popover (their unmapped Kanji note types could feed kanji knowledge), correctable
   furigana.
6. **Later:** code signing for installers (Windows SmartScreen warns); Android (spikes S1/S2,
   ADR 0004) and sync (ADR 0005); Italian/Spanish lemmatizer (ADR 0001); sudachi.rs needs a
   one-line Android patch (ADR 0008); Tauri 3 is in alpha.
