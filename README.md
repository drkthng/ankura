# Ankura

A desktop extensive-reading tool for Japanese that knows your Anki cards. Paste a text and read it
with every word coloured by what Anki says you know. Tap a word for its reading and meaning, and
tell Ankura when you don't know it. Ankura counts what you've seen and looked up, and will use AI
to write new texts around the words you're learning (ROADMAP M2).

Status: early development, milestone M1 of [`docs/ROADMAP.md`](docs/ROADMAP.md). Desktop and
Japanese first (ADR 0009); Android, sync, Italian and Spanish come later.

## Using it

1. Install Anki's [AnkiConnect](https://ankiweb.net/shared/info/2055492159) add-on and keep Anki
   open while Ankura syncs. Ankura only reads from Anki.
2. Start Ankura. In **Settings**, download the Japanese tokenizer dictionary (SudachiDict core,
   about 77 MB) and JMdict, or import any Yomitan dictionary zip you already have.
3. In **Anki**, connect, then for each note type with Japanese words pick the field that holds the
   word (and the reading field, if any). Check the preview, save, and press **Sync now**.
4. In **Library**, paste a text and start reading.

## Developing

Layout:

| Path | What |
|---|---|
| `crates/ankura-core` | The engine: tokenizer, furigana, Anki mapping and sync, dictionary, SQLite store, reader analysis. No UI or platform code. |
| `src-tauri` | Tauri v2 shell: app state and the commands the UI calls. |
| `src` | Svelte 5 UI. |
| `tools/dev` | Development helpers: fake AnkiConnect, dev JMdict builder, end-to-end smoke test. |
| `docs` | Research, architecture options, ADRs, roadmap. |
| `spikes` | Throwaway experiments (S3 tokenizer comparison). |

Requirements: Rust (stable), Node 22+, and on Linux the Tauri prerequisites
(`libwebkit2gtk-4.1-dev build-essential libssl-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev`).

```bash
npm install
npm run tauri dev                # run the app with hot reload
npm run check                    # type-check the UI
cargo test -p ankura-core        # engine tests
ANKURA_SUDACHI_DIC=/path/to/system.dic cargo test -p ankura-core   # also the tests that need Sudachi
npx tauri build                  # installers for the current platform
```

Dictionaries for development: `ANKURA_SUDACHI_DIC` points the app and the tests at a SudachiDict
`system.dic`. `tools/dev/jamdict_to_yomitan.py` builds a JMdict zip from PyPI's `jamdict-data` when
GitHub release downloads aren't reachable.

Without Anki: `python3 tools/dev/fake_ankiconnect.py` serves a small sample collection on port 8765.

End-to-end smoke test (Linux, needs `tauri-driver`, `webkit2gtk-driver` and Xvfb):

```bash
npx tauri build --debug --no-bundle
python3 tools/dev/e2e_smoke.py --sudachi /path/system.dic --dict /path/JMdict.zip --out shots/
```

## Licence

MIT. Dictionaries are downloaded separately and have their own licences: SudachiDict (Apache-2.0),
JMdict (EDRDG, CC BY-SA 4.0).
