# 0023. Italian and Spanish, and the language switch

- Status: accepted for the first step (owner, 2026-11-15: "you can already start implementing the
  stuff for romanic languages … we need a central switch or something"); the details below are
  the implementer's choices and open to change
- Date: 2026-11-15
- Builds on: [0001](0001-target-languages.md) (Japanese, Italian, Spanish), lifts the hold of
  [0009](0009-desktop-japanese-first.md) on Italian and Spanish (desktop only)

## Context

ADR 0001 left the Italian and Spanish lemmatizer as an open research item. The core already
carries a language code in every word key, text and event, and has no Japanese hard-wired outside
the tokenizer, the AI instructions and some heuristics. The app shell passed `"ja"` everywhere.

Research on 2026-11-15 (files fetched and inspected, not from memory):

- **wiktionary-to-yomitan** (`yomidevs`, MIT code; data CC BY-SA 4.0) publishes Italian→English and
  Spanish→English dictionaries in the Yomitan format Ankura already imports:
  `https://huggingface.co/datasets/daxida/wty-release/resolve/main/latest/dict/{it|es}/en/wty-{it|es}-en.zip`
  (17 MB and 22 MB). Besides the entries they hold about 1.06 million Italian and 1.23 million
  Spanish rows for inflected forms, tagged `non-lemma`, each pointing to its word
  (`["case","","non-lemma","",0,[["casa",["plural"]]],0,""]`). Spanish verbs with attached
  pronouns are listed (dímelo, hablarle); Italian ones only partly, as free text.
- Plain form tables (Měchura's lists, ODbL; UniMorph, CC BY-SA 3.0; simplemma's data, mixed
  licences) have no meanings. No established pure-Rust lemmatizer for Italian was found.

## Decision

1. **One database per language.** Japanese keeps `ankura.db`; Italian and Spanish get
   `ankura-it.db` and `ankura-es.db`, made on first use. Texts, Anki field mappings, word
   knowledge, dictionaries, stories, Today, Review, stats and per-language settings (card note
   type and deck, Anki search, voices, goals) are separate without any change to the tables.
   Settings that hold for every language (AnkiConnect address, AI provider, model, effort, mode)
   are copied into a new language's database and written to all of them when changed. API keys
   are in the OS keychain as before. The language used last is remembered in `ankura.db`.
2. **The switch** is a chooser at the top of the navigation. Switching swaps the database and the
   tokenizer and returns to Today. Screens that only exist for Japanese are hidden for the other
   languages: Kanji, Level check, Practice test, the kanji round in Review, the 日本語 meaning tab,
   VOICEVOX.
3. **One dictionary download per language** gives the meanings and the forms. On import, rows
   tagged `non-lemma` go into a form table (`word_forms`, schema v15) instead of the entries.
4. **Tokenizer** (`romance.rs`, behind the `Tokenizer` trait): words are split at spaces,
   punctuation and elisions (l'amico → l' + amico; po', da' stay whole). A word's dictionary form
   is chosen in this order: a grammar word is itself (articles, prepositions, pronouns,
   conjunctions, contractions; they are not counted in coverage, like Japanese particles); a
   candidate the learner has in Anki; one of a dozen very common verbs (era → essere, fue → ir or
   ser as listed); the word itself if it is a headword (sale stays sale); the first word it is a
   form of; then rules that take attached pronouns off verbs (parlarne, dammelo, dímelo,
   vámonos); otherwise the word as written. Capitalised words the dictionary doesn't know are
   names when they aren't first in a sentence. The tokenizer reads the tables through its own
   database connection instead of loading a million forms into memory. The word key's reading is
   empty for these languages.
5. **Writing texts:** Write for me, Today's text and serial stories get instructions for the
   language (`generate::system`), with the length asked for in words.
6. **The other AI features** (cards, questions after a text, translation, simpler sentences and
   versions, interest suggestions) keep their instructions written for Japanese. For another
   language they are adapted in one place, where every request passes (`llm::for_language`, in
   `memory::Recorded`): the language's name replaces "Japanese", and a note says that what is
   specific to Japanese doesn't apply. Good enough to start; instructions written for each
   language can replace this where results are poor.

## Consequences and known gaps

- The first step covers: the switch, the dictionary download, reading with look-ups and
  coverage, Anki field mapping and sync, stats, Review (fill the gap, listen and fill, speed
  match; typed answers ignore capitals and accents), listening with the computer's voices, and
  written texts.
- **Adapted only by the general note of point 6:** the AI instructions for cards, questions after
  a text, translation and help with a sentence, simpler versions, and interest suggestions.
  Their results in Italian and Spanish haven't been looked at.
- **Ambiguous forms are guessed** without looking at the sentence (sale: salt or "goes up";
  Spanish como, para, vino). A wrong guess shows the wrong meaning and counts against the wrong
  word. A correction by the learner, like the reading fixes for Japanese, is the planned answer.
- **Phrases** (per favore, a pesar de) are not matched as one word in these languages.
- Level checks and practice tests for Italian (CILS) and Spanish (DELE/SIELE) are not built
  (`docs/research/proficiency-tests.md`).
- The dictionary's data is CC BY-SA 4.0; it is downloaded by the user, not shipped with Ankura.
- Later sync or Android work (ADR 0005) has to handle several database files, or merge them: the
  language code is still in every row, so merging stays possible.
- Tested with a small made-up dictionary in the core tests. Not run against the real
  dictionaries, a real Anki deck in these languages, or the real AI.
