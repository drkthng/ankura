# 0013. Kanji: pop-up, RTK mnemonics, consistent primitives; editing one mnemonic field

- Status: accepted (owner, 2026-10-27); built 2026-10-27 (point 5: `primitives.rs`, Kanji screen)
- Date: 2026-10-27
- Deciders: owner, via AskUserQuestion
- Amends: ADR 0007 (add-only writes)

## Context

The owner learns kanji with Remembering the Kanji (RTK) and has two kanji note types (Anki:
"Kanji" and "Kanji reverse"). One card asks for the kanji from its keyword, the other for the
keyword from the kanji. The mnemonic (an RTK story built from the kanji's primitives) is on both,
partly hidden, because it's what they need to remember.

They want:

- a kanji pop-up in the reader;
- a new RTK-style story when a kanji is too hard;
- consistent primitive meanings across the whole set. Some components carry two meanings (月 as
  "moon" and as "flesh / body part"); the name a story uses should fit the kanji.

ADR 0007 forbids changing existing notes, and new stories mean exactly that.

## Decision

1. **Kanji pop-up.** Clicking a kanji (in the reader's word pop-up) shows: the user's kanji card
   (keyword, story, Anki state) from the mapped kanji note types, and other words in their
   collection that use it, with their state.
2. **Kanji note mapping.** The user maps their kanji note types like word note types: which field
   holds the kanji, the keyword, and the mnemonic. Both note types can be mapped; a kanji found
   in both is one kanji.
3. **New mnemonic.** For one kanji, the AI writes a new RTK-style story from the kanji's
   primitives and keyword, using the approved primitive names (point 5). The user sees old and
   new and confirms, or asks for another.
4. **Editing is allowed for the mnemonic field only** (amends ADR 0007):
   - only on mapped kanji note types, only the field mapped as the mnemonic;
   - every change is shown old vs new and confirmed by the user, one at a time or as a reviewed
     batch;
   - before writing, Ankura saves the old value in its own database (table of edits: note,
     field, old, new, time), and tags the note `ankura-edited`;
   - every edit can be undone from Ankura (writes the old value back if the field still holds
     Ankura's value);
   - AnkiConnect `updateNoteFields` joins the allow-list, and the client enforces it in code: it
     only sends the one mapped field of a mapped kanji note type.
5. **Consistent primitives:**
   - Ankura reads all kanji stories and lists, per primitive, the names the user uses and where
     they clash.
   - It proposes one name per primitive: the user's own name where they have one, RTK's name as
     the fallback.
   - A component that really is two things gets two names, each used where it fits (月 moon vs
     flesh).
   - The user edits and approves the list. Stories are then rewritten in reviewed batches under
     point 4.
   - RTK names come from the model's knowledge of the books; Ankura ships no copy of Heisig's
     lists.

## Consequences

- The collection is no longer strictly add-only. The risk is limited to one field on one or two
  note types, with a backup and undo for every change.
- Writing stories needs the automatic AI mode (API key).
- Primitive analysis reads every kanji note once (a few thousand); the result is stored and
  reused.
