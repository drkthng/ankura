# 0004. Anki data sources: AnkiDroid provider on the phone, AnkiConnect on desktop, .colpkg import as fallback

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

AnkiConnect doesn't exist on Android. The research (`docs/research/claims-check.md` #6, #7, #10)
found:

- **AnkiDroid 2.24.0+** (May 2026) has a card search URI,
  `content://com.ichi2.anki.flashcards/cards`, whose `selection` is an Anki search. It returns
  interval, reps, lapses, type, queue, and FSRS stability, difficulty, decay and last review time.
  Permission `com.ichi2.anki.permission.READ_WRITE_DATABASE` (runtime prompt).
- **AnkiConnect `cardsInfo`** has no FSRS fields. Anki's `prop:s`/`prop:r` searches can bucket cards
  by FSRS state.
- **Anki's automatic backups** (`.colpkg`, a zstd-compressed SQLite file) contain full FSRS state
  and are safe to read. The live `collection.anki2` is locked exclusively by Anki.
- The Anki sync protocol is internal and not meant as a data API.

## Decision

- **Android:** read AnkiDroid's ContentProvider directly (2.24 or later). Incremental refresh with
  `rated:N` and `added:N` searches. Compute FSRS retrievability on the device.
- **Android fallback:** import a `.colpkg`/`.apkg` exported from AnkiDroid or Anki, when AnkiDroid
  is older than 2.24 or the permission is denied.
- **Desktop:** AnkiConnect (`findCards`, `cardsInfo`, `cardsModTime`), with FSRS state from
  `prop:s`/`prop:r` search buckets. Optional exact mode reads the newest automatic backup.
- **Later:** a knowledge snapshot from the desktop through Ankura's sync (ADR 0005), so the phone
  has data even without AnkiDroid.
- **Not doing:** implementing the Anki sync protocol, or reading the live collection file.

## Consequences

- Ankura on Android requires AnkiDroid 2.24+ for live data. The UI must explain this, and offer the
  import path, when the provider lacks the columns or the permission is denied.
- AnkiDroid debug and parallel builds use different package names; support at least the release
  package `com.ichi2.anki`, and make the package configurable.
- The knowledge model has to accept two levels of precision: exact R (Android, exact mode) and
  bucketed R (AnkiConnect searches).
- Spike S2 measures provider query speed on a large collection before the design is final.
