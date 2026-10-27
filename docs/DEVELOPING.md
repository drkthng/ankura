# Developing Ankura

[← README](../README.md)

Ankura is a Tauri v2 app: a Rust engine (`crates/ankura-core`), a thin Tauri shell (`src-tauri`) and
a Svelte 5 UI (`src`). Design decisions are in [`adr/`](adr), the plan in [ROADMAP.md](ROADMAP.md),
and the state of things for whoever picks up next in [NEXT-SESSION.md](NEXT-SESSION.md).

Layout:

| Path | What |
|---|---|
| `crates/ankura-core` | The engine: tokenizer, furigana, Anki mapping and sync, dictionary, SQLite store, reader analysis, AI texts and cards, stats. No UI or platform code. |
| `src-tauri` | Tauri v2 shell: app state and the commands the UI calls. |
| `src` | Svelte 5 UI. |
| `tools/dev` | Development helpers: fake AnkiConnect, dev JMdict builder, end-to-end smoke test. |
| `docs` | User guides (`guide/`), research, architecture options, ADRs, roadmap. |
| `spikes` | Throwaway experiments (S3 tokenizer comparison). |

Requirements: Rust (stable), Node 22+, and the Tauri prerequisites for your system:

- **Windows:** Rust and Microsoft's C++ build tools. WebView2 ships with Windows 10 and 11.

  ```powershell
  winget install --id Rustlang.Rustup
  winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  # then, in a new terminal:
  rustup default stable-msvc
  ```

- **macOS:** `xcode-select --install`, then Rust from <https://rustup.rs>.
- **Linux (Debian/Ubuntu):** Rust from <https://rustup.rs>, plus
  `libwebkit2gtk-4.1-dev build-essential libssl-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev`.

The first build compiles all dependencies and takes several minutes.

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
Without an AI key: `python3 tools/dev/fake_llm.py` stands in for the Anthropic API on port 8766; start
the app with `ANKURA_LLM_URL=http://127.0.0.1:8766 ANKURA_API_KEY=test`.

End-to-end smoke test (Linux, needs `tauri-driver`, `webkit2gtk-driver` and Xvfb). It drives the
whole app against the fake AnkiConnect and fake AI, and saves screenshots:

```bash
npx tauri build --debug --no-bundle
python3 tools/dev/e2e_smoke.py --sudachi /path/system.dic --dict /path/JMdict.zip --out shots/
```

Every push to `main` runs the **Build** workflow: type-check, engine tests, and installers for
Windows, macOS and Linux as downloadable artifacts.
