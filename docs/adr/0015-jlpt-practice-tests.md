# 0015. JLPT-style practice tests per level

- Status: accepted (owner, 2026-10-27)
- Date: 2026-10-27
- Deciders: owner, via AskUserQuestion
- Research: [`docs/research/proficiency-tests.md`](../research/proficiency-tests.md) §1, §5,
  Recommendations v1.5. Follows [0014](0014-proficiency-level-check.md).

## Context

The level check (ADR 0014) estimates a level. The owner's next request is a practice test for one
chosen level that feels like the real JLPT: the same question types (大問) in the same order, the
same times, and a pass/fail-style verdict. Official questions are copyrighted; Ankura writes its
own. Listening comes later (TTS), so the practice tests have no listening section.

## Decision

1. **One test per level, N5-N1, without listening.** The layout follows the JLPT's sections and
   question types. Item counts are the JLPT's approximate targets (目安), which change between
   sittings; Ankura's table (`practice::layout`) is a close copy, not an official one:

   | Level | Parts and minutes | Question types |
   |---|---|---|
   | N5 | 文字・語彙 20 · 文法・読解 40 | 漢字読み 7, 表記 5, 文脈規定 6, 言い換え類義 3 · 文法形式 9, 文の組み立て 4, 文章の文法 4, 短文 2, 中文 2, 情報検索 1 |
   | N4 | 25 · 55 | 7, 5, 8, 4, 用法 4 · 13, 4, 4, 短文 3, 中文 3, 情報検索 2 |
   | N3 | 30 · 70 | 8, 6, 11, 5, 5 · 13, 5, 5, 短文 4, 中文 6, 長文 4, 情報検索 2 |
   | N2 | 言語知識・読解 105 | 5, 5, 語形成 3, 7, 5, 5, 12, 5, 5, 短文 5, 中文 9, 統合理解 2, 主張理解 3, 情報検索 2 |
   | N1 | 110 | 漢字読み 6, 文脈規定 7, 言い換え類義 6, 用法 6, 10, 5, 5, 短文 4, 中文 9, 長文 4, 統合理解 2, 主張理解 4, 情報検索 2 |

2. **Full or half length**, picked at the start. Half keeps every question type with half the
   items (at least one, or one passage) and half the time.
3. **Timer:** official minutes per part, shown counting down. When time is up Ankura says so; the
   learner can submit or keep going, and the result marks the part as overtime. The timer can be
   turned off. Time used is stored, so a test can be left and resumed.
4. **Items.** Ankura picks the target words from the bundled JLPT lists (ADR 0014) at the test's
   level, so the AI writes around Ankura's word choices instead of recalling prep books.
   - 漢字読み: the AI writes a sentence with the word; the options are made by rule (voicing,
     long vowels, small っ), and a distractor that the dictionary knows as a reading of that
     spelling is dropped.
   - 表記, 語形成, 文脈規定, 言い換え類義, 用法, 文法形式: written by the AI from a structured
     spec (type, level, target word), JSON output.
   - 文の組み立て: the AI writes a sentence and the four fragments in order; Ankura shuffles them
     and asks for the one in the ★ slot (third of four).
   - 文章の文法 and all reading types: a passage with its questions; reading questions the
     checking call can answer without the passage are dropped.
   - Every AI item goes through the checking call of ADR 0014 (answer cold, flag ambiguous);
     items it gets wrong are dropped. Ankura asks for about a third more than it needs, and asks
     once more if a type comes up short. A type that is still short is used with fewer items.
   - When the tokenizer is set up, sentences with list words two or more levels above the test's
     level (other than the tested word) are dropped (level drift, research §5).
   - Every item carries a short English explanation (why the key is right, why the others
     aren't), written with the item.
5. **Preparation:** the whole test is written before it starts, with progress, several AI calls at
   a time (about 15-25 calls for a full N1 test). Preparation can be stopped and continued.
6. **Scoring** (not the JLPT's: its scores are IRT-scaled and can't be reproduced):
   - Scoring sections as on the score report: N1-N3 言語知識 (vocabulary + grammar) 0-60 and 読解
     0-60; N4/N5 言語知識・読解 0-120. Each is the share right, scaled to the range.
   - Pass line without listening: the official pass mark scaled to the sections tested (N1
     100/180 → 67/120; N2 90 → 60; N3 95 → 63; N4 90 → 60/120; N5 80 → 53/120), plus the
     sectional minimums (19 of 60; 38 of 120).
   - Verdict: **likely pass** at or above the line + 6% of the tested maximum with every section
     at its minimum or above; **unlikely** below the line − 6% or under any sectional minimum;
     **borderline** in between.
   - Reference grades A/B/C (67% and more, 34-67%, under 34%, as on the official score report)
     for 文字・語彙 and 文法; accuracy per question type.
   - The result says that it is an estimate and that listening isn't included.
7. **Review:** every question with the learner's answer, the key and the explanation. The learner
   can **report a bad question** (stored; reported items don't count in the score, which is
   recalculated). Words of missed vocabulary items can be **sent to Anki cards** (the ADR 0012
   draft flow). **Retry missed questions** starts a short untimed round with only those items,
   with no AI calls.
8. **Storage:** the `tests` / `test_items` tables of ADR 0014 with kind `practice` (and `retry`),
   plus `tests.config` (level, length, timer, time used per part) and `test_items.reported`
   (schema v11).

## Consequences

- A full test costs more AI calls than a level check (about four times as many for N1); the half
  test about half that. Opus at medium effort takes a few minutes to prepare a full test.
- Generated items are checked by the model that wrote them. That catches broken keys, not every
  unnatural sentence; the report button and stored answers are the second line.
- The verdict rests on raw shares, not on calibrated difficulty; a test whose AI items are easier
  than the real exam's will look like a pass. The result screen says so.
