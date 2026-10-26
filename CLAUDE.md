# Ankura: agent instructions

Ankura is a cross-platform (desktop + Android) extensive-reading tool for language learners. It
knows the user's Anki card stats, tracks its own word/kanji exposure and lookup stats, and uses AI
to generate interest-tailored n+1 texts with a high share of known words.

**Status:** no code yet. Start with `docs/HANDOFF.md`, which covers the vision, requirements,
open decisions, and the first phase of work. `docs/research/prior-ai-report.md` is an unverified
research report written by a weaker model. Treat it as leads to check, not as fact.

## Working rules (from the owner's global config; cloud sessions don't see that file)

- **Commit and push straight to `main`.** No branches or PRs unless the owner explicitly asks
  for one in that session (owner's instruction, 2026-10-26). If they do ask for a PR, it's a
  draft (`gh pr create --draft`), and the owner marks it ready for review.
- **Questions go through `AskUserQuestion`:** 2-4 options each, recommended option first and
  labeled "(Recommended)", `multiSelect: true` when the choices aren't exclusive. Don't end a
  turn with prose questions the user has to answer by typing. Put the recap and analysis in
  text first, then make the tool call.
- Write in plain, direct English. No marketing language in docs.

## About the owner

- Has used Anki for close to 20 years, so the collection is large, old, and has many different
  note types. Any Anki integration has to cope with that (configurable field mapping, not
  assumptions).
- Wants the tool to be **fun and easy to use**. UX matters as much as the engine does.
- Mentions kanji, so Japanese is very likely a target language. Confirm which languages are in
  scope before designing the tokenizer layer.

## Conventions (to be filled in once the stack is chosen)

- Stack: Tauri v2, Rust core, Svelte UI (`docs/adr/0003-app-stack.md`), pending spikes S1-S3 in
  `docs/architecture-options.md` §9. v1 languages: Japanese, Italian, Spanish (ADR 0001).
- Build / test / lint commands: _none yet_
- Record architecture decisions in `docs/adr/NNNN-title.md`.
