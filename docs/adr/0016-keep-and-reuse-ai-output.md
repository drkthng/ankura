# 0016. Keep everything the AI writes, and reuse it

- Status: accepted (owner, 2026-10-27)
- Date: 2026-10-27
- Deciders: owner ("everything produced by our agents should be put into a database, so that we
  later do not need to produce so much by AI"; for test prep "the user can say how much shall be
  reused and how much completely new")

## Context

Practice tests (ADR 0015) and the level check (ADR 0014) ask the AI for about a third more
questions than they need and threw the extras away; every new test was written from scratch.
"Simplify this sentence" and card sentences asked the AI again for input it had seen before.
Generated texts, kanji stories and primitive analyses were already stored.

## Decision

1. **A log of every AI request and answer** (`ai_log`: kind, system prompt, prompt, answer or
   error, duration). The app wraps its AI client in `memory::Recorded`, so every feature is
   covered, including ones added later. The log is the raw record; reuse works from the
   structured stores below.
2. **A question bank** (`item_bank`) for practice tests and the level check. Every question that
   passes the checks goes in, whether a test uses it or not. Questions on one passage share a
   group and are reused together. Each bank question counts its uses; a reported question is
   never reused. Test items point to their bank question (`origin`).
3. **Practice tests: the learner chooses the mix** when starting a test:
   - *Bank first* (default): as many questions from the bank as it has, the AI writes the rest;
   - *Half new*: at most half from the bank;
   - *All new*: no reuse (the new questions still go into the bank).

   By default only questions not used in a test before are reused. *Include questions from
   earlier tests* lets used ones come back, least used and longest ago first; with a big enough
   bank a test is then ready at once without AI requests. Before starting, the screen says how
   many questions come from the bank and how many the AI writes. One question per word per test
   still holds.
4. **The level check reuses unseen bank questions only**, with no setting: a level estimate from
   questions the learner has answered before would be biased.
5. **Cached answers** (`ai_cache`), reused only if they still pass the known-word check against
   what the learner knows now:
   - a simpler sentence, by the original sentence;
   - card sentences, by word. "New sentence" on a card always asks the AI (the learner wants a
     different one); the answer is kept as well.
6. **Savings are counted** (`ai_reuse`) and shown in Settings with the size of the log, the bank
   and the cache.
7. **Existing tests fill the bank** on upgrade (schema v12): their questions count as used once.

Not reused (yet): generated texts (the library is the store; offering unread texts before
writing a new one is a possible next step), card field filling (depends on the sentence and the
note type), kanji stories (a new story is asked for because the old one didn't work), interest
suggestions.

## Consequences

- Fewer AI requests over time; a learner who takes several tests at one level mostly takes
  questions written for earlier tests. With *Include questions from earlier tests*, scores can
  rise from memory rather than skill; the option is off by default.
- Passage checks now cover the extra passages too (two checking requests each), which costs a
  little more per new test and pays back when the passages are reused.
- The log grows with use (a few KB per request); nothing deletes it yet.
