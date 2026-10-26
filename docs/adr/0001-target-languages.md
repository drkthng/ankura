# 0001. Target languages for v1: Japanese, Italian, Spanish

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

Ankura needs a tokenizer and dictionary per language, and Japanese needs extras (kanji tracking,
furigana). HANDOFF §5 asked whether the design should be language-agnostic from day one.
`docs/architecture-options.md` §7 recommends a language-agnostic core with Japanese first.

## Decision

v1 supports **Japanese, Italian and Spanish**.

- The core is language-agnostic. Every key, text and event carries a language code. Tokenizers sit
  behind one interface that returns surface, lemma, optional reading, part of speech and offsets.
- Japanese gets the script extras: kanji sets per key, kanji state, furigana, optional vertical
  text.
- Italian and Spanish use a space-delimited tokenizer plus a **lemmatizer**. Inflection is heavy
  (verb conjugation, clitics such as *dímelo*, *dammelo*), so splitting on spaces isn't enough.

## Consequences

- Japanese tokenizer choice is settled by spike S3 (Lindera + IPADIC vs UniDic vs Sudachi).
- **Open research item:** a lemmatizer for Italian and Spanish that runs on the device, in Rust,
  under a licence compatible with MIT. Candidates to check: lookup tables derived from Wiktionary
  or from simplemma's data (check the data licence), Hunspell/morphology dictionaries, or a
  spaCy-derived lookup table. Clitic splitting needs its own rules. Not researched in phase 1.
- Dictionaries: Yomitan-format files cover Japanese (JMdict) and, through Wiktionary-derived
  Yomitan dictionaries, Italian and Spanish too (to be confirmed).
- Ankura has to run three languages through every feature (field mapping, coverage check, LLM
  prompts) from the start. Keeping tests per language avoids Japanese-only assumptions.
