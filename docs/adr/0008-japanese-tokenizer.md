# 0008. Japanese tokenizer: Sudachi with the core dictionary

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion, after spike S3

## Context

Spike S3 (`spikes/s3-tokenizers/REPORT.md`) compared Lindera + IPADIC, Lindera + UniDic and
sudachi.rs + SudachiDict on a 150-sentence gold set. All three got every lemma right where they
segmented as expected, and about 97% of surface readings.

They differ in three ways:

- **Compounds:** Sudachi core in mode C kept 58 of 61 compounds whole; every other configuration
  kept 45.
- **Spelling variants:** Sudachi and UniDic unify them (分かる/わかる, 子供/子ども); IPADIC doesn't.
- **Size:** IPADIC is a 7 MB download (45 MB on disk); Sudachi core is 45 MB (193 MB on disk,
  137 MB peak memory).

Ankura matches tokens against a large, old Anki collection, so compounds and variants matter more
than size.

## Decision

- Use **sudachi.rs** with **SudachiDict core** for Japanese, on desktop and Android.
- Use **mode C** units for display and tapping, and their **mode A** parts (`Morpheme::split`) as
  extra match candidates against Anki cards.
- Use `normalized_form` (plus the reading) as the word key, so spelling variants share
  telemetry and Anki matches.
- The same dictionary version runs on every device, so keys agree across sync. The dictionary
  version is stored with the data, and a version change triggers a re-key.
- Ship the dictionary as a first-run download (about 45 MB compressed), not inside the APK.
- Pin sudachi.rs to a git commit (not on crates.io; 0.7 is marked unstable).
- Until upstream fixes it, carry a one-line patch that adds `target_os = "android"` to
  `sudachi/src/plugin/loader.rs`, and send that fix upstream.
- Keep Sudachi behind Ankura's tokenizer interface, with Lindera + IPADIC as the documented
  fallback.

## Consequences

- Needs about 193 MB of disk and up to about 140 MB of mapped memory on the phone. Spike S1 should
  confirm this is acceptable on a real device.
- The tokenizer doesn't solve context-dependent readings (十分, 開いている, 人気のない, この方) or
  counter sound changes (三匹). The reader therefore needs:
  - correctable furigana,
  - the Anki card's reading where a card matches,
  - a small table of counter sound changes.
- Dictionary updates change keys for a small number of words. The re-key step has to merge
  telemetry rows that collapse into one key.
- SudachiDict is Apache-2.0, compatible with Ankura's MIT licence; the NOTICE must be shipped.
