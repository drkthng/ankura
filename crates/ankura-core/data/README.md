# Bundled data

- `jlpt_vocab.tsv`, `jlpt_kanji.tsv`: JLPT word and kanji lists by Jonathan Waller
  ([tanos.co.uk](https://www.tanos.co.uk/jlpt/)), licensed Creative Commons BY. Parsed into JSON
  by [Bluskyo/JLPT_Vocabulary](https://github.com/Bluskyo/JLPT_Vocabulary) (MIT); converted to TSV
  for Ankura, dropping entries with notes in the headword ("(", "=", "〜"). Used as a difficulty
  guide for the level check (ADR 0014). They are not official JLPT lists; there are none since 2010.
