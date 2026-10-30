# 0021. Simpler versions of a text that is too hard

- Status: accepted (owner, 2026-10-31)
- Date: 2026-10-31
- Deciders: owner ("for texts where I do not know 95% of the words … have the app produce one or
  multiple simpler texts based on exactly the text I pasted where each text has 95% known words
  but taken all new texts together all new words from the new text are still covered")
- Research: [`docs/research/learning-evidence.md`](../research/learning-evidence.md) §1 (coverage)

## Decision

1. **Plan, no AI** (`adapt::plan`): the original is split into sections of about 600 characters
   (whole paragraphs). Its unknown words (word tokens Anki and the learner's marks don't know;
   names excluded) are listed in order of appearance, each once for the whole text. Each
   section's new words are cut into groups of about 5% of the section's word count, 2-8 words.
   One group is one version.
2. **Writing** (`adapt::write_version`): a version is a Write-for-me text whose request carries
   the original section (`GenerateRequest.adapt`): retell it in simpler Japanese, same content,
   events and order, the group's words used in the meaning they have in the original, nothing
   added. Same coverage check and retries as Write for me; length about the section's; です/ます if
   the original uses it. Saved with `adapted_from`, `part`, `of` and the words it missed.
3. **Coverage of the new words** (`adapt::status`): the words the versions actually used are
   compared with the original's new words; the rest are shown, and `write_missing` writes more
   versions for them, retelling the sections where they appear.
4. **In the reader:** texts under 95% (not versions or episodes themselves) show the offer, the
   plan on request, "Write the versions" (one request after another, with progress), the
   versions written, and the words not used yet. A version links back to the original. The plan
   the reader showed is the one written, so parts keep their numbers.

## Consequences

- A short text with many new words gets many versions retelling the same content; the guide says
  so. Long texts are retold part by part.
- About one AI request per version, plus retries; nothing for the plan.
- "Known" is what Anki and the marks say now; while Anki has many overdue cards, versions can
  still feel harder than 95%.
