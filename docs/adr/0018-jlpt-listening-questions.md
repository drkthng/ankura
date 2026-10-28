# 0018. JLPT listening questions in practice tests

- Status: accepted (owner, 2026-10-28; "just continue" with the agreed order: reader first,
  then tests)
- Date: 2026-10-28
- Follows: [0015](0015-jlpt-practice-tests.md) (practice tests), [0017](0017-listening.md)
  (voices), [0016](0016-keep-and-reuse-ai-output.md) (question bank)

## Context

Practice tests had no 聴解 (listening) section, so the verdict used a pass line scaled down to
two sections. Ankura can now speak Japanese (ADR 0017). The JLPT's listening types are 課題理解,
ポイント理解, 概要理解 (N3-N1), 発話表現 (N5-N3, needs pictures), 即時応答 and 統合理解 (N2,
N1, long multi-part audio).

## Decision

1. **Listening is optional** ("Include 聴解" when starting a test). With it, the test has a third
   (N5-N3) or second (N2, N1) part, 聴解, with the official minutes (N5 30, N4 35, N3 40, N2 50,
   N1 55).
2. **Question types:** 課題理解, ポイント理解, 概要理解 (N3-N1) and 即時応答, with the JLPT's
   approximate counts for those types (N3: 6, 6, 3, 9). 発話表現 and 統合理解 are left out for
   now.
3. **Scripts:** the AI writes each question as a script: the narrator's introduction, the lines
   of a woman and a man, the narrator's question, the options and an English explanation. The
   check answers from the transcript; 課題理解 and ポイント理解 questions it can answer without
   the talk are dropped. A level-drift check runs on the spoken lines.
4. **What is shown:** for 課題理解 and ポイント理解 the four options are printed (as in the test
   booklet); for 概要理解 and 即時応答 the options are only spoken ("１。…"), and the sheet shows
   their numbers. 即時応答 has three options, replied by the other speaker.
5. **Voices:** narrator = the chosen voice; the woman and the man get different VOICEVOX
   characters (from lists of known female and male characters) or different system voices,
   else the same voice at a higher or lower pitch.
6. **Playback:** with the timer on, a question plays when it is opened, once, as in the exam;
   without the timer it can be replayed. The review shows the transcript with "Listen again".
7. **Scoring** with listening: the JLPT's sections (N3-N1: 言語知識, 読解, 聴解, 0-60 each; N5,
   N4: 言語知識・読解 0-120, 聴解 0-60), the official pass mark out of 180, and the sectional
   minimums (19; 38 for N5/N4 language knowledge and reading).
8. Listening questions go into the question bank like the others.

## Consequences

- A test with listening costs about two more AI requests per listening type, plus the audio
  (VOICEVOX: synthesized once per line and kept; system voices: free).
- Without VOICEVOX, dialogues by one system voice at two pitches are harder to follow than the
  real exam.
- The minutes are the official ones although 発話表現 and 統合理解 are missing, so the part is a
  little more relaxed than the real one.
