# 0003. App stack: Tauri v2, Rust core, Svelte UI

- Status: accepted, subject to spikes S1-S3
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

`docs/architecture-options.md` §8 compares three options: Tauri v2 + Rust + web UI, Kotlin
Multiplatform + Compose, and Flutter + a Rust core. The reading screen is the core of the product.
For Japanese it needs furigana (`<ruby>`) and ideally vertical text. Web engines do both; Compose
and Flutter would need custom layout. Tauri's Android support is the least mature of the three
(91 open Android issues as of 2026-10-26, Tauri 3 in alpha).

## Decision

- **Shell:** Tauri v2 (currently 2.12.x) on Windows, macOS, Linux and Android.
- **Core:** Rust, shared by both platforms: tokenizers, field normalization, knowledge model,
  coverage check, SQLite (rusqlite), AnkiConnect and LLM HTTP clients, sync batches.
- **UI:** Svelte (with TypeScript).
- **Android:** a Kotlin Tauri plugin for the AnkiDroid ContentProvider (see ADR 0004).

## Consequences

- Before scaffolding the full app, run spikes S1 (Tauri Android + AnkiDroid permission and query),
  S2 (query speed on a large collection) and S3 (tokenizer comparison and APK size). If S1 fails
  badly, the fallback is Flutter with the same Rust core (option C), which keeps the core
  unchanged.
- The Rust core stays independent of Tauri (a plain library crate), so the fallback stays cheap
  and the core can be tested without a UI.
- Plan for a Tauri 2 → 3 migration at some point.
- Android work to budget for: back-button handling, keyboard insets, and the permission flow.
- Linux uses WebKitGTK; test the reader there early.
