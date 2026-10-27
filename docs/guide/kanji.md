# Kanji

[← README](../../README.md)

If you learn kanji with their own Anki cards, the way Remembering the Kanji (RTK) teaches them (a
keyword and a mnemonic story built from the kanji's parts), Ankura can show those cards while you
read and help when a story doesn't stick.

![A kanji's card in the word pop-up, with a new story to compare](../img/kanji.png)

## Setting up

On the **Anki** screen, under **Kanji notes**, choose your kanji note type from *Add a kanji note
type…*. Ankura suggests which field holds the kanji, the keyword and the mnemonic; check them and
**Save**. Map both note types if you have two (say, kanji → keyword and keyword → kanji). A kanji
in both counts once, and its strongest card decides how well you know it. Then **Sync now**.

## In the reader

The word pop-up lists the kanji of the word. Click one to see:

- its **keyword** and how well Anki says you know it;
- its **story**, hidden until you click *Show story* (as on your cards, so you can test yourself
  first);
- **your words** that use this kanji, from your Anki collection.

## A new mnemonic

When a story doesn't work for you, press **New mnemonic**. The AI writes a new RTK-style story from
the kanji's primitives. It uses the primitive names from your other stories, RTK's names where you
have none, and the meaning that fits this kanji where a component has two (月 as moon or as
flesh). The story keeps your stories' language and formatting. You see your story and the new one
side by side, with the primitives it used:

- **Save to Anki** writes it into the mnemonic field of this kanji's notes, in all mapped note
  types;
- **Another** asks for a different one; **Keep mine** closes it.

## What Ankura may change

This is the one place Ankura changes notes you already have, so it's deliberately narrow:

- only the field you mapped as the **mnemonic**, only on your mapped **kanji note types**;
- only when you press **Save to Anki**;
- before writing, Ankura keeps the old story in its own database, and tags the note
  `ankura-edited` (find them in Anki with `tag:ankura-edited`);
- **Undo Ankura's change** puts the old story back. If you've edited the story in Anki since,
  Ankura leaves your version alone.

Decision record: [ADR 0013](../adr/0013-kanji-and-rtk-mnemonics.md). Coming next: making
primitive names consistent across your whole set.
