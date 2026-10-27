# Roadmap

Scope: desktop app, Japanese only (ADR 0009). Each milestone ends with something the owner can
use.

## M1: Read with your Anki knowledge

Paste a Japanese text and read it with every word coloured by what Anki says you know.

- [x] Workspace: `crates/ankura-core` (no Tauri dependency), Tauri v2 shell, Svelte 5 UI.
- [x] Tokenizer: Sudachi core, mode C units with mode A parts (ADR 0008), behind the
      `Tokenizer` trait.
- [x] Text library: paste, title, list, open, delete. SQLite.
- [x] Reader: tokens as tappable spans, furigana on kanji (toggle), comfortable Japanese
      typography.
- [x] AnkiConnect client (`AnkiSource` trait): note types, fields, `findNotes`/`findCards`,
      `notesInfo`, `getIntervals`, card state via `is:` searches and FSRS buckets via
      `prop:s`/`prop:r` searches. (`cardsInfo` renders every card's HTML, too slow for a large
      collection.)
- [x] Field-mapping screen: per note type, target field and optional reading field, with a live
      preview of the normalized word and a count of failures.
- [x] Field normalization: HTML, furigana brackets, cloze, sound tags, lists.
- [x] Sentence cards: the word is the bold/italic/underlined part of a sentence field; the reading
      field's first line is its reading. "Suggest fields" detects this from the notes.
- [x] Knowledge model: unknown / new / learning / known / fading per word key, highest card wins
      (architecture-options §3).
- [x] Coloured reader, plus a comprehension bar (share of known tokens) before and while reading.
- [x] Word popover: reading, dictionary gloss (JMdict), Anki card state, "I don't know this".
- [x] Telemetry: exposures (paragraph visible for 3 s) and lookups as events with a device ID.
- [x] First-run setup: download SudachiDict core and JMdict, connect to AnkiConnect.

Known gaps after M1, to pick up next:

- [x] Try it on the owner's real collection and texts; tune field suggestions and defaults
      (sentence cards, unmarked sentences, light verbs, suspended cards, overdue recall without FSRS).
- [ ] The JMdict download (GitHub release) hasn't been run from the build environment, which
      can't reach GitHub release downloads; importing a Yomitan zip is tested.
- [ ] Phrase cards (several words in one field) are stored but not yet matched in the reader.
- [x] Basic function words (する, こと, とき, この…) count as known by default (ADR 0010).
- [ ] Incremental Anki sync (`cardsModTime`) instead of a full re-read.

## M2: Generated texts

- [x] LLM providers: Anthropic and OpenRouter (ADR 0006, ADR 0010), key in the OS keychain.
- [x] Generate a text from target words, topic, length and style, with the deterministic coverage
      check and retries (architecture-options §6). Target suggestions: looked-up, fading, learning
      and new words.
- [x] Example sentences for target words.
- [x] Copy & paste through claude.ai, no API key needed (ADR 0011).
- [x] Show words in the spelling the user knows (card or text), not the normalized key.
- [x] "Simplify this sentence" (word pop-up: simpler version with known words, coverage-checked,
      translation on request).

## M3: Personal and fun

Owner's order (2026-10-27): stats, kanji, furigana, interest profile, then the proficiency test.

- [x] Anki cards from reading: card list, i+1 sentences, fields in the style of the user's notes,
      add-only (ADR 0007, ADR 0012).
- [x] Select a phrase in the reader to look it up and card it as one unit (ADR 0012).
- [x] Phrases from Anki cards and the user's marks show as one word in the reader (気になります
      matches a 気になる card).
- [x] Stats: characters read per day, streaks, words met and looked up, "stealth leeches" (known
      in Anki, looked up again while reading) and most looked-up words, each with "+ card".
- [x] Kanji (ADR 0013): kanji note mapping, kanji pop-up (card, keyword, story, words with it),
      new RTK-style mnemonics; edits limited to the mnemonic field, confirmed, backed up, undoable.
- [x] Kanji: consistent primitive names across the set (ADR 0013 point 5): analyse stories,
      approve names, rewrite stories with review.
- [x] Correctable furigana: counter sound changes by rule (三匹 さんびき), and reading corrections per
      text or everywhere from the pop-up (ADR 0008 consequences). Card readings: not yet.
- [x] Interest profile: weighted topics and topics to avoid, suggested from what the user reads,
      editable on the Write page; steers topic ideas, picks the topic when none is given, and
      the avoid list goes into every text prompt.
- [x] Proficiency test v1: a JLPT-style level check (ADR 0014): yes/no words with made-up words,
      kanji readings, two-stage AI grammar, reading; latent-level result with CEFR reference.
- [ ] JLPT practice tests per level (v1.5); Italian (CILS) and Spanish (DELE/SIELE) tests;
      listening with TTS. Research in `docs/research/proficiency-tests.md`.

## M4: Release

- [ ] Installers for Windows, macOS, Linux.
- [ ] Imports: AnkiMorphs known-morphs CSV, Lute term CSV.

## Later (not started)

- Android: spikes S1 and S2, AnkiDroid provider (ADR 0004), sync (ADR 0005).
- Italian and Spanish (ADR 0001).
