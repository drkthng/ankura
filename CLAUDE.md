# Ankura: agent instructions

Ankura is a cross-platform (desktop + Android) extensive-reading tool for language learners. It
knows the user's Anki card stats, tracks its own word/kanji exposure and lookup stats, and uses AI
to generate interest-tailored n+1 texts with a high share of known words.

**Status:** desktop app for Japanese. M1 and M2 in daily use with the owner's API key. M3
(cards, phrases, stats, kanji/RTK, furigana fixes, interests, level check, JLPT practice tests,
kept and reused AI output, listening in the reader and in practice tests, Today with serial stories and questions, translation in context, simpler versions;
ADRs 0012-0021) built
and tested with fakes; the owner is trying them on their collection. **Start with `docs/NEXT-SESSION.md`** (state, owner, code
map, cloud-session tips, next steps), then `docs/ROADMAP.md` and the ADRs in `docs/adr/`.
`docs/HANDOFF.md` is the original phase-1 brief (vision, requirements); its tasks are done.
`docs/research/prior-ai-report.md` is an unverified report by a weaker model; treat it as leads to
check, not as fact.

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

- Stack: Tauri v2, Rust core, Svelte UI (`docs/adr/0003-app-stack.md`), pending spikes S1-S2 in
  `docs/architecture-options.md` §9 (S3 done: `spikes/s3-tokenizers/REPORT.md`).
- v1 languages: Japanese, Italian, Spanish (ADR 0001). Japanese tokenizer: sudachi.rs +
  SudachiDict core (ADR 0008).
- Layout: `crates/ankura-core` (engine, no UI or platform code), `src-tauri` (Tauri shell and
  commands), `src` (Svelte 5 UI), `tools/dev` (fake AnkiConnect, dev JMdict, e2e smoke test).
- Build / test: `npm install`, `npm run tauri dev`, `npm run check`, `cargo test -p ankura-core`
  (set `ANKURA_SUDACHI_DIC` to a SudachiDict `system.dic` to include the tokenizer tests). E2E:
  `npx tauri build --debug --no-bundle` then `tools/dev/e2e_smoke.py` (see `docs/DEVELOPING.md`).
- Record architecture decisions in `docs/adr/NNNN-title.md`.
- User docs: `README.md` (overview, getting started) links to one guide per feature in
  `docs/guide/`. Update the matching guide when a feature changes.
