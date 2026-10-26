# 0005. Sync of Ankura's own data: mergeable per-device batches, LAN pairing first

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

Ankura's own data (texts, exposures, lookups, word status overrides, interests, settings) lives on
both desktop and phone. Research (`docs/architecture-options.md` §5) found that the off-the-shelf
sync engines either need a server (ElectricSQL, PowerSync, Turso) or are dormant (cr-sqlite). The
official Syncthing Android app was discontinued in December 2024.

## Decision

- **Data model:** mergeable by construction.
  - Counts are per-device counters (summed on read).
  - Events are append-only per device.
  - Overrides, interest weights and settings are last-writer-wins per key, by hybrid logical clock.
  - Texts are immutable by UUID.
  - The Anki knowledge snapshot is written only by the desktop.
- **Batches:** each device exports its changes since the last sync as a compressed batch file; the
  other device applies it.
- **First transport: LAN pairing.** Pair once with a QR code, then sync over the local network
  (mDNS discovery) when both apps are open.
- **Later transports:** WebDAV and a shared folder, using the same batch files.

## Consequences

- No accounts, servers or third parties in v1. Sync only happens when both devices are on the same
  network, which is acceptable because both work offline.
- Pairing needs a shared secret from the QR code, and the traffic needs encryption and
  authentication (for example a pinned key per pair).
- Because the batch format doesn't depend on the transport, adding WebDAV or a folder later
  doesn't touch the data model.
- Android may need a foreground service only while a sync is running; no long background work
  (claims-check #11).
