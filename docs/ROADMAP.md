# Roadmap

Scope: desktop app, Japanese only (ADR 0009). Each milestone ends with something the owner can
use.

## M1: Read with your Anki knowledge

Paste a Japanese text and read it with every word coloured by what Anki says you know.

- [ ] Workspace: `crates/ankura-core` (no Tauri dependency), Tauri v2 shell, Svelte 5 UI.
- [ ] Tokenizer: Sudachi core, mode C units with mode A parts (ADR 0008), behind the
      `Tokenizer` trait.
- [ ] Text library: paste, title, list, open, delete. SQLite.
- [ ] Reader: tokens as tappable spans, furigana on kanji (toggle), comfortable Japanese
      typography.
- [ ] AnkiConnect client (`AnkiSource` trait): decks, note types, fields, `findCards`,
      `cardsInfo`, FSRS buckets via `prop:s`/`prop:r` searches.
- [ ] Field-mapping screen: per note type, target field and optional reading field, with a live
      preview of the normalized word and a count of failures.
- [ ] Field normalization: HTML, furigana brackets, cloze, sound tags, lists.
- [ ] Knowledge model: unknown / new / learning / known / fading per word key, highest card wins
      (architecture-options §3).
- [ ] Coloured reader, plus a comprehension bar (share of known tokens) before and while reading.
- [ ] Word popover: reading, dictionary gloss (JMdict), Anki card state, "I don't know this".
- [ ] Telemetry: exposures (paragraph visible for 3 s) and lookups as events with a device ID.
- [ ] First-run setup: download SudachiDict core and JMdict, connect to AnkiConnect.

## M2: Generated texts

- [ ] LLM providers: Anthropic and OpenRouter (ADR 0006), key in the OS keychain.
- [ ] Generate a text from target words and a level, with the deterministic coverage check and
      retries (architecture-options §6).
- [ ] Example sentences for target words.
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
