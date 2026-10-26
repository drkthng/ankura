# 0009. Build the desktop app for Japanese first

- Status: accepted
- Date: 2026-10-26
- Deciders: owner (session instruction: "concentrate on the working desktop app for Japanese
  first, mobile support and Italian/Spanish later, except if it is absolutely necessary at this
  stage")

## Context

ADRs 0001-0008 describe a desktop + Android app for Japanese, Italian and Spanish. The owner wants
a working desktop app for Japanese before anything else. Spikes S1 and S2 (Android) and the
Italian/Spanish lemmatizer research are on hold.

## Decision

Until the owner says otherwise, work goes into the desktop app (Windows, macOS, Linux) for
Japanese only. No Android code, sync, or Italian/Spanish support.

The few things that are cheap now and expensive to retrofit are kept:

| Kept now | Why it can't wait | Cost now |
|---|---|---|
| Rust core as its own crate, with no Tauri or desktop dependency | Android (ADR 0003) reuses the core; untangling it later means a rewrite | A crate boundary |
| Anki access behind an `AnkiSource` interface | AnkiDroid (ADR 0004) becomes a second implementation | One trait |
| Language code in every word key and text | Adding a language later would otherwise be a schema migration of all telemetry | One column |
| UUID primary keys, a device ID, and an append-only event log for exposures and lookups | Sync (ADR 0005) needs per-device, mergeable data; counters without device IDs can't be merged later | A few columns |
| Tokenizer interface (ADR 0001) | Italian and Spanish plug in here | One trait |

Everything else for mobile, sync and other languages waits.

## Consequences

- ADRs 0001, 0003, 0004 and 0005 still describe the target. Only their timing changes.
- Spikes S1 and S2 move to the Android phase, so Tauri's Android maturity stays unverified. If it
  fails then, the Rust core moves to the Flutter fallback (ADR 0003).
- Dictionaries (SudachiDict core, JMdict) are downloaded on first run on the desktop.
