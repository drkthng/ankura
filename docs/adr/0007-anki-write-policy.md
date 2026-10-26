# 0007. Writing to Anki: add-only in v1

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

Creating cards from what the user reads (word, sentence, reading, gloss) is a core feature of the
tools in the landscape (Yomitan, asbplayer, anki.koplugin, jidoujisho). The owner's collection is
nearly 20 years old, so any change to existing notes carries real risk.

## Decision

Ankura may **add new notes** to Anki and must never modify or delete existing notes, cards, decks
or note types.

- The user chooses the target deck and note type, and maps Ankura's fields onto it. Ankura may
  create its own note type if the user asks.
- Desktop: AnkiConnect `addNote` (with `canAddNotes` for duplicate checks).
- Android: AnkiDroid's provider/AddContentApi insert, under the same permission as reading.
- On the phone, notes created while AnkiDroid isn't available go into an offline queue and are sent
  later (anki.koplugin does the same).
- Every note Ankura adds gets a tag (for example `ankura`), so the user can find or remove them in
  Anki.

## Consequences

- Features such as "add this sentence to my existing card" wait for a later decision.
- The Anki client code needs a write allow-list: only add and duplicate-check calls may be used.
  This should be enforced in code, not just by convention.
- Ankura has to map its fields onto note types the user already has, so field-mapping UI is
  needed for writing as well as reading.
