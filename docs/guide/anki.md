# Connecting Anki

[← README](../../README.md)

Ankura reads your collection through the [AnkiConnect](https://ankiweb.net/shared/info/2055492159)
add-on. Anki has to be open while Ankura syncs or adds cards.

## 1. Connect

The **Anki** screen connects automatically to AnkiConnect's default address. If you changed
AnkiConnect's port, enter the address there.

## 2. Tell Ankura where the word is

Every collection is different, so Ankura asks for each note type that holds Japanese words:

- **Which field holds the word.** Either the whole field is the word (a vocabulary card), or
  the word is the **bold, italic or underlined part** of a sentence (a sentence card).
- **The reading field**, if there is one. Ankura takes its first line as the word's reading.

**Suggest fields** looks at your notes and proposes this, with a reason. **Preview** shows what
Ankura reads from a sample of notes: single words, phrases, or nothing. Nothing is saved until you
press **Save**. Note types you leave alone are skipped.

Ankura copes with most field formats: HTML, furigana in `漢字[かんじ]` or ruby form, cloze
deletions, audio tags, lists. A sentence with nothing marked still works when the reading field
names one of its words.

## 3. Sync

**Sync now** reads your cards and works out what you know. Sync again after studying.

- **Known:** interval (or FSRS stability) of at least 21 days.
- **Fading:** known, but Anki's recall estimate is below 80%. Ankura uses FSRS's own estimate
  when FSRS is on; otherwise it estimates from the interval and how overdue the card is.
- **Learning** and **new** come straight from Anki.
- **Suspended cards don't count** by default. Change this, and the two thresholds, in Settings.
- You can limit Ankura to part of your collection with an Anki search (e.g. `deck:日本語*`).
- When several cards exist for one word, the strongest wins.

The sync report shows, per note type, how many notes gave a word, a phrase, or nothing.

## Does Ankura change my collection?

Ankura only ever **adds** notes, when you make [cards](cards.md), and tags them `ankura`. It never
changes or deletes existing notes, cards, decks or note types. The AnkiConnect client refuses any
other kind of request in code. (A planned kanji feature will be allowed to change one mnemonic
field you choose, with your confirmation and an undo; see
[ADR 0013](../adr/0013-kanji-and-rtk-mnemonics.md).)
