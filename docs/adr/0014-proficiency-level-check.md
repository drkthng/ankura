# 0014. Proficiency test v1: a JLPT-style level check

- Status: accepted (owner, 2026-10-27)
- Date: 2026-10-27
- Deciders: owner, via AskUserQuestion
- Research: [`docs/research/proficiency-tests.md`](../research/proficiency-tests.md)

## Context

The owner wants a proficiency test: a general one built on how proficiency is best measured, and
tests modelled on official exams (JLPT for Japanese; CILS/CELI and DELE/SIELE later), listening
last. Official JLPT questions are copyrighted and can't be copied; the app has to write its own.
There are no official word lists since 2010.

## Decision

1. **v1 is one level check for Japanese**, 15-20 minutes. JLPT practice tests per level come
   next. Italian, Spanish and listening come later.
2. **Word and kanji levels come from Jonathan Waller's JLPT lists** (tanos.co.uk, CC BY; taken
   from the Bluskyo/JLPT_Vocabulary parse, MIT): 8,138 words and 2,211 kanji tagged N5-N1. They
   ship with Ankura (`crates/ankura-core/data/`), with the credit shown on the test screen. They
   serve as a difficulty guide, not as a claim about the official test.
3. **Sections:**
   - **Vocabulary, yes/no:** 40 words from the lists (8 per level) that Anki doesn't already say
     the user knows, plus 16 made-up words (two kanji from the list that form no dictionary
     word). Saying yes to made-up words lowers the vocabulary score.
   - **Kanji readings:** 15 words (3 per level); the reading among four options. Distractors are
     made by rule (voicing, long vowels, small っ, other readings), not by the AI.
   - **Grammar, two stages:** 8 fill-the-blank items at the starting level, then 12 one level
     easier, the same or harder, depending on the first stage. The AI writes them. A second AI call
     answers each item without the key, and items it gets wrong or finds ambiguous are dropped.
   - **Reading:** 2 short passages, 2 questions each, at the level estimated so far. Questions the
     AI can answer without the passage are dropped.
4. **Starting level:** the user picks N5-N1, or lets Ankura start from their Anki knowledge
   (shown as the share of each level's words known in Anki).
5. **Scoring:** a latent-level model. For each possible level (below N5, N5 … N1), every answer
   has a probability given the gap between the item's level and the learner's level. Multiplying
   these gives a probability for each level. The result shows:
   - the most likely level and the probability of each level;
   - accuracy per section and level;
   - a vocabulary estimate (Anki-known words plus the tested share of the rest);
   - the Japan Foundation's CEFR reference for that level (N5 A1, N4 A2, N3 A2-B1, N2 B1-B2,
     N1 B2-C1; reading and listening only).

   It is labelled provisional and suggests a retest after a month.
6. **Every test and every answer is stored** (schema v10), so a later version can calibrate items
   and adapt item by item.

## Consequences

- No calibrated item bank: probabilities come from assumed response rates per level gap, not from
  data. The result says so.
- Each test costs a handful of AI calls (two grammar batches and their checks, one reading batch
  and its check).
- The word lists are dated (based on the pre-2010 lists); the N3 boundary is the least reliable.
