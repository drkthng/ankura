# 0019. Today: a daily session, serial stories, questions after a text

- Status: accepted (owner, 2026-10-30)
- Date: 2026-10-30
- Deciders: owner, via AskUserQuestion (chose "Today + backlog rescue" and "Serial stories +
  questions" from the ranked list)
- Research: [`docs/research/feature-ideas.md`](../research/feature-ideas.md) #1, #2, #6, #7;
  [`docs/research/learning-evidence.md`](../research/learning-evidence.md) §2, §4, §7

## Context

Ankura's parts (practice words, Write for me, reader with listening, cards) sat on separate
screens. The research: most words need about 10 spaced encounters, retrieval right after reading
helps, and the owner's Anki has a large backlog of overdue cards after a six-month break. Ankura
can't change Anki's schedule (ADR 0007).

## Decision

1. **Today** is the first screen. Five steps: today's words → today's text → read, then listen →
   questions → cards. Progress is kept per local day (`today_sessions`, schema v13).
2. **Backlog rescue:** today's words (`today::practice_words`, 6) are about half fading Anki words
   (overdue cards; the best-established first, which are the easiest to save), then a stealth
   leech, a word missed in questions, a word looked up, and words being learned. Words that were
   targets in the last day wait. The learner can remove, add or swap words. Today shows how many
   fading words there are and how many the learner met in a text this week. Reading a word never
   counts as an Anki review.
3. **Serial stories** (`series`, `series.rs`): a series has a title, a topic, length, style and a
   story bible (characters and setting, written by the AI with episode 1 and kept). An episode is
   an ordinary generated text with `series`, `episode` and `summary` in its extra JSON. The next
   episode's prompt has the bible, the last three summaries, and up to three earlier target words
   (from the last two episodes, the least used first), added to the day's words. Deleting a series
   keeps its episodes as texts.
4. **Questions after a text** (`quiz.rs`): two comprehension questions and a cloze on up to two of
   the text's target words. Checked like the tests' items (ADR 0014): right with the text,
   comprehension questions not answerable without it. Kept in the bank (ADR 0016, test
   `text_quiz`, grouped by text), so a text asks the AI once. Each answer gets its explanation at
   once; missed cloze words are recorded (`quiz_missed` events) and come back in the next days'
   words. Available at the end of every text in the reader, not only from Today.
5. The reader opened from Today shows the session's steps: Done reading, Listen (done when the
   read-aloud reaches the end), Questions, Back to Today.

## Consequences

- One AI request more per text (the questions, plus two checking requests), once per text.
- The session is a guide, not a lock: every step can be done in any order or skipped.
- Episodes reuse the coverage check of Write for me, so carried-over words count as targets and
  don't lower coverage.
