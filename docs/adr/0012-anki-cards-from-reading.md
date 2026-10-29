# 0012. Anki cards from reading: a card list, notes in the user's own style

- Status: accepted (owner, 2026-10-26)
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion

## Context

ADR 0007 allows Ankura to add notes to Anki, never to change them. The owner wants cards for
words met while reading, in the style of their existing notes, and wants to decide which ones go
to Anki. Their main note type ("Japanisch") is a sentence card: the word is bold in the sentence,
the reading field holds the word's reading on the first line and the sentence's reading after it,
plus a definition field. Other users' note types will differ, so a fixed layout won't do.

## Decision

1. **A card list comes first.** Every card starts as a draft in Ankura's card list ("Cards"
   screen). The user edits, discards or adds drafts; "Add to Anki" sends the chosen ones. A
   setting ("Add cards without the list") sends a card straight to Anki as soon as it's filled.
2. **Where drafts come from:**
   - the word pop-up in the reader, with two choices: "this sentence" (the sentence the word
     appears in) or "a new sentence";
   - a generated text's target words (their sentence in the text);
   - words the user marked "I don't know this" (listed on the Cards screen, each with a new
     sentence);
   - the words picked on the Write page ("Make cards": one new sentence per word, no text).
3. **Phrases:** selecting several words in the reader looks them up as one phrase (気になります →
   気になる): the last content word goes to its dictionary form, and the form the dictionary knows
   wins (that one, or the selection as written). The phrase then works like a word: pop-up,
   marks, cards.
4. **New sentences are i+1:** the AI writes one sentence around the word in which every other word
   is known. Ankura checks it with the same tokenizer check as generated texts (only the target
   may be unknown) and asks again with feedback, up to three tries, keeping the best.
5. **Fields copy the user's own notes.** The user picks a note type and deck once in Settings.
   To fill a draft, Ankura reads the three newest notes of that type from Anki and gives them to
   the AI as examples, together with the facts: the word, its reading, the sentence, the
   sentence's reading from the tokenizer, and every dictionary entry with its senses (up to 4
   entries, 8 senses each). The AI picks the meaning the word has in this sentence; the first
   sense is often the wrong one (気になる: "to weigh on one's mind" vs "to be interested in"). The AI answers with one
   value per field, keeping the examples' HTML (bold, line breaks). Fields that hold media
   (`[sound:…]`, `<img>`) stay empty. The user can edit every field before adding.
6. **Adding** uses AnkiConnect `canAddNotesWithErrorDetail` then `addNote`, tag `ankura`.
   AnkiConnect's duplicate check (same first field) applies; a duplicate is reported, not
   forced. The AnkiConnect client only allows these two write actions (ADR 0007).
7. Filling fields and new sentences need the automatic AI mode (API key). In copy & paste mode
   the card list still works with "this sentence", but fields have to be filled by hand.

## Consequences

- Each filled card costs one AI call, each new sentence one to three. With the owner's API credits
  this is negligible.
- The AI can get readings wrong; the tokenizer's sentence reading is given as a hint and the user
  sees the card before it's added (unless they turn on adding without the list).
- Drafts live in Ankura's database (schema v5), so the list survives restarts.

## Update 2026-10-29: change a card on request

Owner's request: while checking cards, quickly ask the AI for a specific change (for ほど: an
example of ほど…ない instead of the meaning on the card). Each draft has a request box
(`cards::revise`, command `card_change`): the AI gets the facts (word, dictionary entries,
sentence reading), the current sentence and fields, known words and the request, and returns the
sentence and all fields in the same format (made-up audio/images removed). A draft without fields
changes only its sentence and is then filled as usual. Undo restores the previous version
(`card_restore`). Source label "changed with AI".
