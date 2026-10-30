# 0020. Translation in context, and meanings in simple Japanese

- Status: accepted (owner, 2026-10-31)
- Date: 2026-10-31
- Deciders: owner ("a translation of a sentence that makes sense in the context, kind of exactly
  how we create our Anki cards"; "switch between the English meaning and the meaning in (simple)
  Japanese")

## Context

The word pop-up had English glosses (JMdict) and "Simplify this sentence" (with a translation of
the original behind a click). The owner wanted a translation that fits the text around the
sentence, like the meaning picked for a card, and a Japanese definition of words. JMdict has no
Japanese definitions; good monolingual dictionaries are not free to ship.

## Decision

1. **Translate this sentence** (`explain::translate`): the AI gets the sentence, the paragraph
   around it, and the clicked word, and returns a translation of that sentence as meant in
   context, plus one line on what the word means there. Kept by sentence and word (`ai_cache`
   kind `translate`).
2. **English | 日本語** in the pop-up, remembered per device. 日本語 shows:
   - entries from imported dictionaries whose glosses are in Japanese (a monolingual Yomitan
     dictionary the learner owns);
   - an AI explanation in simple Japanese of the meaning in this sentence (`explain::explain_ja`):
     one or two sentences in the style of a learner's dictionary, with the learner's known words.
     Checked like generated texts: under 90% known words, the AI is asked once more with the
     unknown words; shown with furigana on unknown words. Kept by word and sentence
     (`explain_ja`).

## Consequences

- One AI request the first time per sentence (translation) or per word and sentence
  (explanation); none afterwards.
- The explanation's words are shown with furigana but don't open their own pop-ups (a pop-up in
  a pop-up); the reader stays the place for looking words up.
