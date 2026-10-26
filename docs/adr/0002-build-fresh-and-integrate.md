# 0002. Start a fresh codebase; integrate with other tools through protocols and formats

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

HANDOFF §5 asked whether to fork or extend Lute, build on LinguaCafe, or start fresh.
`docs/architecture-options.md` §4 lays out the case for each:

- **Lute** (MIT) is a Python/Flask server used through a browser. It has no offline Android story
  and doesn't read Anki stats.
- **LinguaCafe** (GPL-3.0) needs Docker with MySQL, Redis and a Python service, runs on x64 only,
  and would force a licence change.
- **AnkiMorphs** and **jidoujisho**, the nearest tools on Anki knowledge and Android, were archived
  in October 2026.

## Decision

Start a fresh codebase under the MIT licence. Integrate through protocols and data formats, not
shared code:

- Anki: AnkiConnect on desktop, the AnkiDroid ContentProvider on Android, `.apkg`/`.colpkg` import.
- Dictionaries: Yomitan dictionary format.
- Imports for existing users: AnkiMorphs `known_morphs-*.csv`, Lute's term CSV export
  (`lute/term/routes.py`).

Code may be reused only from MIT/BSD/Apache/public-domain sources (for example the FSRS libraries,
Lute, Lukaisu, Jiten, LWT). GPL/AGPL projects (AnkiMorphs, asbplayer, VocabSieve, Yomitan, LinguaCafe)
are design references only.

## Consequences

- Everything is built from scratch, including what Lute already has: term editing, text library,
  multi-word terms. The reader UI is the largest piece.
- Ankura can be designed local-first on both platforms, with Anki knowledge and AI generation at
  the centre.
- Contributors must not paste code from GPL/AGPL projects. A note in CONTRIBUTING (once it exists)
  should say so.
