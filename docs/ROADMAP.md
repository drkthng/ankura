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
- [ ] "Simplify this sentence".

## M3: Personal and fun

- [ ] Interest profile: inferred from pasted texts, editable (weights, custom scenarios), steers
      generation.
- [ ] Stats: words seen, lookups, friction, "stealth leeches", reading streaks and character
      counts.
- [ ] Add a word to Anki (add-only, ADR 0007).
- [ ] Kanji popover: other words in your deck with the same kanji.
- [ ] Correctable furigana, card readings, counter sound changes (ADR 0008 consequences).

## M4: Release

- [ ] Installers for Windows, macOS, Linux.
- [ ] Imports: AnkiMorphs known-morphs CSV, Lute term CSV.

## Later (not started)

- Android: spikes S1 and S2, AnkiDroid provider (ADR 0004), sync (ADR 0005).
- Italian and Spanish (ADR 0001).
