# 0022. Review: Ankura's own schedule, game rounds, and answers sent to Anki

- Status: accepted (owner, 2026-11-15)
- Date: 2026-11-15
- Deciders: owner, via AskUserQuestion ("present stuff that I'm about to forget, maybe a bit more
  gamified than just boring flashcards. If at all possible would like this info to go back to
  anki too"). Chosen: Anki words and Ankura words; all four round types, Fill the gap first;
  score, combo and a daily goal; right answers only go to Anki.
- Amends: [0007](0007-anki-write-policy.md) (add-only). Replaces the "no schedule, not Anki"
  limits of idea #10 and the rejection of idea #31 in
  [`docs/research/feature-ideas.md`](../research/feature-ideas.md).
- Research: [`docs/research/learning-evidence.md`](../research/learning-evidence.md) §2 (spacing),
  §4 (retrieval practice)

## Context

Ankura knew which Anki words were fading and put some into the day's text, but it had no schedule
of its own. Words without an Anki card (looked up, missed in questions) were only events. Nothing
done in Ankura reached Anki, so the owner's backlog (845 cards 30+ days overdue at the last sync)
only shrank through reviews in Anki itself.

AnkiConnect has `answerCards` (answers a card with an ease, through Anki's own scheduler) and
`areDue`. Both were checked in the add-on's source (`plugin/__init__.py`), not against the
owner's installed version.

## Decision

1. **Review** is a screen of its own, and a warm-up card on Today. A round has up to 10 items.
2. **What is due** (`review.rs`), in this order of picking: items on Ankura's schedule that are
   due (about half the round); words met in Ankura and never reviewed (marked "I don't know
   this", missed in a text's questions, or known in Anki but looked up twice or more in 90 days);
   words fading in Anki, the best-established first. Words marked known or ignored are left out.
   "Practise anyway" fills a round with items that aren't due.
3. **Ankura's own schedule** (`fsrs.rs`, tables `review_items`, `review_log`, schema v14): FSRS-5
   with its default weights, three grades. The first answer to a word Anki schedules starts from
   the card's strength (interval or FSRS stability), so an old card isn't treated as new. A word
   on Ankura's schedule that isn't due is not asked, whatever Anki says. The schedule is kept per
   word key, and per kanji.
4. **Fill the gap:** the word is taken out of a sentence. Sources, in order: texts where the
   learner met the word (exposure events), other texts they opened, then the sentence of their
   own Anki card (sync stores it for sentence cards, table `review_cards`). No sentence: the
   meaning alone is the cue. The dictionary's first sense is shown under the sentence. The answer
   is typed: the form in the sentence or the dictionary form, in kanji, kana or romaji
   (`kana::romaji_to_hiragana`), so no Japanese keyboard is needed. Helpers: a hint (first kana
   and length), four choices, "I don't know". No AI request is made in a round.
5. **The other rounds:** Listen and fill (the sentence is read aloud, the meaning hidden until
   the answer), Speed match (five words and their meanings per board, a clock), Kanji round
   (kanji → keyword and keyword → kanji with four choices, the learner's story after each).
6. **Three results:**

   | Result | When | Ankura's schedule | Anki |
   |---|---|---|---|
   | recalled | typed in Fill the gap, no hint | Good | "Good" on the word's card |
   | helped | right with a hint, from choices, heard, matched first try | Hard, back within 3 days | nothing |
   | missed | wrong, "I don't know", a wrong pairing | Again, back within a day | nothing |

7. **Answers sent to Anki** (`review::send_anki`, table `anki_answers`): for a recalled word,
   "Good" is queued for each of its review cards that tests that one word (a card whose field
   gives one word or one phrase; a phrase card is not answered for the words inside it), and
   that isn't suspended or buried. When the round ends, and before every sync, each queued card
   that is still due in Anki (`areDue`) is answered through `answerCards`, so Anki's scheduler
   sets the next date, exactly as a review in Anki would. Cards that aren't due are left alone.
   Answers wait up to a day when Anki isn't running, then are dropped. At most one answer per
   card in 20 hours. Every answer is logged and listed on the Review screen. A setting turns
   sending off (`review_anki`).
8. **Game layer:** points (10 recalled, 5 helped, plus 2 per right answer in a row, up to 10
   extra), a combo counter, a daily goal of right answers (default 20, `review_goal`) with a
   streak of days that met it, and "rescued in Anki" (cards answered this week). No levels,
   currency or leaderboards (ideas #27-28 stay rejected).
9. **Words without a card:** the result screen offers "+ card" for them, through the existing
   card drafts (ADR 0012), so they get Anki's treatment from then on.

## Consequences

- ADR 0007 no longer holds as written: besides adding notes (0007) and the kanji mnemonic field
  (0013), Ankura now changes the schedule of existing cards, but only by answering due cards
  "Good" after a typed recall. The allow-list in `anki.rs` gains `areDue` and `answerCards`.
  Ankura still never answers "Again", never reschedules directly, and never touches cards that
  aren't due.
- The recall asked for in Fill the gap (meaning and context → word) is harder than the owner's
  recognition cards, so a "Good" from Ankura is not easier to earn than one in Anki. It is still
  a different test than the card's own; the owner accepted that.
- A card answered from Ankura shows in Anki's review history as a normal review with a very short
  answer time.
- Until the next sync, a word whose card was answered is set to "known" locally.
- Kanji answers never go to Anki (they are picked from choices).
- Reading a word still doesn't count as a review, in Anki or in Ankura.
- Not tried against the owner's Anki yet: whether their AnkiConnect version has `answerCards`
  (an old one answers "unsupported action"; Ankura then says to update the add-on and keeps the
  answers waiting).

## Addendum, 2026-11-15: looking up words of the sentence

Owner: "in the review, since we do not have romaji, I would like to be able to click a word and
see its reading and meaning in the context". A round item now carries the sentence as reader
tokens (`before_tokens`, `after_tokens`), and the Review screen opens the reader's word pop-up on
a click. Implementer's choices: a look-up of another word is not help with the gap (the answer
still counts as recalled and can go to Anki); it is recorded as a look-up event of the clicked
word; the pop-up gets the full sentence (translation, simpler version, card) only after the
answer, because the sentence holds it.

## Addendum, 2026-11-16: shorter sentences, readings in Speed match

Owner: the Fill the gap sentences "are a bit too long … it takes too long and is no fun
anymore", and in Speed match "I'd like to have the ability to see the reading of the word too".

- **Sentences are cut, not rewritten** (`review::window`): no AI request, and the words are
  still the ones the learner read. A sentence wider than `MAX_WIDTH` (60; kana and kanji count
  2, so about 30 Japanese characters) is cut to the word's clause, from comma to comma, plus
  neighbouring clauses while they fit, the following one first. A clause that is too long by
  itself loses words at its ends; a very short one gets a neighbour (`MIN_CONTEXT`). The item
  says where it was cut (`cut_before`, `cut_after`: shown as …) and carries the whole sentence
  (`sentence`), which is shown after the answer and used for the pop-up's translation and for
  cards. Listen and fill reads the cut part. Text sentences up to 200 characters are now
  candidates (was 90), since they are cut anyway.
- **Readings in Speed match:** the reading stands above a kanji word (ruby) once the word is
  picked; a "Readings" box shows all of them from the start (kept in the browser's local
  storage, not in the database: a display preference of this computer). The answer still
  counts as "right, with help", as all Speed match answers do.
