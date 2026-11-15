# Languages: Japanese, Italian, Spanish

[← README](../../README.md)

Ankura can be switched between **Japanese**, **Italian** and **Spanish** with the chooser at the
top of the navigation. Italian and Spanish are new: the basics work, some AI features are still
worded for Japanese (see below).

## Each language is separate

Every language has its own texts, Anki field setup, known words, stories, Today, Review, stats
and card settings. Switching changes all of them at once, and nothing of one language shows up in
another. Your AI setup and the Anki connection are shared.

## Setting up Italian or Spanish

1. Choose the language at the top left.
2. **Settings → Dictionaries → Download the dictionary.** One download (about 20 MB, made from
   Wiktionary) gives the meanings for look-ups and the forms of every word (*case* → *casa*,
   *hablaba* → *hablar*). Importing takes a minute or two. Ankura can't read a text before this.
3. **Anki → map your note types** for that language, as for Japanese: tell Ankura which field
   holds the word, or that it is the bold word in a sentence. An article in front of the word
   (*il gatto*, *la casa*) is fine. If your collection mixes languages, limit the sync
   to that language's decks with the Anki search on the same screen (for example
   `deck:Italiano*`). Then **Sync now**.
4. **Settings → Listening:** Ankura uses your computer's voices for the language. On Windows,
   add one under Settings → Time & language → Speech if none is listed.

## What works

- **Reading:** words coloured by what Anki says you know, look-ups, coverage. Articles,
  prepositions, pronouns and conjunctions count as grammar, not as vocabulary.
- **Write for me, Today's text and serial stories**, written in the language.
- **Review:** fill the gap, listen and fill, speed match. Typed answers don't need capitals or
  accents (`citta` counts for *città*).
- **Stats, cards list, listening** with the computer's voices.

## What doesn't yet

- **Kanji, Level check and Practice test** are for Japanese only and are hidden.
- **Cards from the AI, questions after a text, translating a sentence, simpler versions:** they
  run, but their instructions still say the text is Japanese, so results may be off. Being
  adapted next.
- **Words with two meanings by form** are guessed without the sentence: *sale* can be "salt" or
  "goes up", Spanish *vino* "wine" or "came". If you have an Anki card for one of them, Ankura
  picks that one.
- **Phrases** of several words aren't recognised as one yet.

## Where the data is

Japanese is in `ankura.db`, Italian in `ankura-it.db`, Spanish in `ankura-es.db`, all in Ankura's
data folder (shown at the bottom of Settings).
