# Prior AI research report (UNVERIFIED)

> Written by a less capable model in answer to the owner's research question (see
> `docs/HANDOFF.md` §1). Preserved here with the formatting cleaned up and the content intact.
> **Every claim needs checking.** The claims list is in HANDOFF.md §4. The original numbered
> citations pointed to an unresolved source list (readwithlira.com, libhunt.com, lector.dev,
> github.com, reddit.com, ankiweb.net, trendshift.io, linux.do, youtube.com, ycombinator.com) and
> have been dropped.

## 1. Tools covered

**Lute v3 (jzohrab/lute-v3).** Python (Flask), SQLite, SQLAlchemy, Jinja. Pluggable parsers:
MeCab (Japanese), Jieba (Chinese), spaCy or regex (space-delimited languages). Word statuses 1-5,
"Well Known", "Ignored", including multi-word terms. Tracks reading sessions. Anki export via
CSV/TSV or AnkiConnect. Weaknesses: localhost web server, so mobile use needs self-hosting,
Termux, or similar. No awareness of Anki intervals or stability.

**LWT (HugoFara/lwt) and "Lukaisu".** Classic PHP/MySQL with Docker support in community
branches. Claims that "Lukaisu" is a local-first Kotlin Android client. No SRS intelligence, no
Anki sync, no AI.

**AnkiMorphs (mortii/anki-morphs, successor to MorphMan).** Anki add-on using spaCy and CAMeL
Tools. Extracts morphemes (lemma and part of speech) from every card and treats them as known
based on review stats (for example interval > 21 days). Scores sentences to find n+1 (exactly one
unknown morph).

**Jidoujisho (lrorpilla/jidoujisho).** Flutter Android immersion suite (video, audiobooks, text).
Creates and checks cards through the AnkiDroid API (ContentProvider and Intents).

**read-bridge (WindChimeEcho/read-bridge).** TypeScript web/Electron app that uses LLMs for n+1
reading material and target-language explanations.

**Yomitan.** Browser extension for dictionary lookups. AnkiConnect export.

| Tool | Tech | Form factor | Anki sync | Parsing | Own tracking | AI |
|---|---|---|---|---|---|---|
| Lute v3 | Python/Flask/SQLite | Self-hosted web | One-way (AnkiConnect export) | MeCab, Jieba, spaCy, regex | Status 1-5, reading stats | None |
| AnkiMorphs | Python/spaCy | Anki add-on | Direct DB | spaCy/CAMeL | Uses intervals/lapses | None |
| Jidoujisho | Dart/Flutter | Android APK | AnkiDroid ContentProvider | Local tokenizers | Lookup history | Minimal |
| Yomitan | TypeScript ext | Browser | One-way (AnkiConnect) | Deinflection | Via add-ons | None |
| read-bridge | TypeScript | Web/Electron | Manual export | NLP/LLM | Reading position | LLM n+1 |

## 2. Proposed architecture

Local-first clients plus a sync layer:

- **Desktop:** Tauri v2 (Rust core) with a React or Svelte UI. Rust/SQLite engine, Lindera or
  MeCab tokenizer, telemetry, AnkiConnect at `http://127.0.0.1:8765`.
- **Android:** Tauri v2 with a Kotlin bridge, the same web UI, the same Rust engine, and the
  AnkiDroid ContentProvider through ContentResolver.
- **Sync:** CRDT, ElectricSQL, or self-hosted sync for exposure logs, interest profiles, and
  cached Anki knowledge snapshots.
- Flutter is the alternative (proven by Jidoujisho), but MeCab would need native `.so`
  libraries.

**Tokens** resolve to: `surface` (食べた), `lemma` (食べる), `reading` (たべた), `kanji_set`
(["食"]).

**Desktop Anki bridge (AnkiConnect):**
`{"action":"findNotes","params":{"query":"deck:Japanese"}}`, then
`{"action":"cardsInfo","params":{"cards":[...]}}`, which is claimed to return interval, reps,
lapses, factor, and FSRS stability and difficulty.

**Android Anki bridge:** permission `com.ichi2.anki.permission.READ_WRITE_DATABASE`, query
`content://com.ichi2.anki.provider/cards` (the authority is doubtful, see HANDOFF §4 #6).

**Classification:** Known = interval ≥ 21d (or FSRS stability ≥ 30). Learning = in queue with
interval < 21d. Unseen = not in any deck. Leech = lapses > 4.

**Telemetry schema:**

```sql
CREATE TABLE vocabulary_telemetry (
    lemma TEXT PRIMARY KEY,
    surface_sample TEXT NOT NULL,
    pos TEXT,
    exposure_count INTEGER DEFAULT 0,  -- each time the word renders
    lookup_count INTEGER DEFAULT 0,    -- each tap/click
    last_seen_timestamp DATETIME,
    user_status INTEGER DEFAULT 0      -- 0 unmarked, 1 learning, 2 known, 3 ignored
);
CREATE TABLE kanji_telemetry (
    kanji TEXT PRIMARY KEY,
    exposure_count INTEGER DEFAULT 0,
    lookup_count INTEGER DEFAULT 0,
    last_seen_timestamp DATETIME
);
```

**Friction** = lookup_count / exposure_count. High friction on an Anki-mature word marks a
"stealth leech".

## 3. AI pipeline

Inputs: target vocabulary, user interests, known-vocabulary constraints. Steps: build the context
(target level such as JLPT N3, token budget, banned lemmas, theme from interests), generate with
an LLM (OpenRouter, Ollama, or Anthropic) returning JSON (title, text, per-target-word sentence
plus translation), then verify: re-tokenize, compute the comprehensibility index
CI = (N_known + N_target) / N_total, and simplify or regenerate if CI < 0.95.

**Interest profiling:** TF-IDF or embeddings on pasted texts feed an `interest_profile` table
with exponential decay, W_topic = Σ_k e^(−λ(t − t_k)). A manual tag cloud with weights
(Sci-Fi +2.0, Horror −1.0) and custom scenario prompts ("a detective novel set in 1920s Osaka").

Example prompt: N3 Japanese story about urban exploration and antique clock restoration that
must use 歯車, 錆びる, and 精密, keep 95% of the vocabulary at or below level, run 350-500
words, and return JSON.

## 4. Feature ideas

- Kanji deconstruction popover (other deck words with the same kanji).
- Comprehensibility heatmap or bar before reading.
- Exposures only count after a paragraph has been in the viewport for 3 or more seconds.
- One-click context-rich Anki cards (sentence, gloss, pitch accent/furigana, TTS via Piper or
  Edge-TTS).
- "Simplify sentence" AI rewrite.
- Inject Anki leeches into generated stories.

## 5. Roadmap and caveats

1. MVP: Tauri v2 desktop and Android, SQLite schema, tokenizer, a reading UI with colored token
   spans.
2. Anki bridge: Rust AnkiConnect client and a Kotlin AnkiDroid ContentProvider bridge; map cards
   to lemmas.
3. AI: provider client, JSON n+1 generation, verification loop, interest UI.

Caveats: Anki locks `collection.anki2` while running, so use AnkiConnect or the ContentProvider
rather than reading the file. Use normalized lemmas as the join key, because Yomitan cards often
store inflected forms. Android 14+ battery optimization kills long background work.
