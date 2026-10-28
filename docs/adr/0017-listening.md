# 0017. Listening: texts read aloud

- Status: accepted (owner, 2026-10-28)
- Date: 2026-10-28
- Deciders: owner, via AskUserQuestion
- Research: [`docs/research/proficiency-tests.md`](../research/proficiency-tests.md) §6

## Context

Listening is next in the owner's order (after practice tests). It serves reading (hearing a text
while reading it) and, later, JLPT listening questions (聴解). Options for the voice: the
computer's own voices, VOICEVOX (free, offline, natural Japanese voices; a separate program),
or a paid cloud voice.

## Decision

1. **Voices:** the computer's Japanese voices through the webview's Web Speech API by default;
   **VOICEVOX when it runs** (setting "VOICEVOX when it runs, else this computer", the default).
   The learner can also pick "This computer" or "VOICEVOX only", a computer voice, a VOICEVOX
   voice (speaker and style), the VOICEVOX address and the speed. No cloud voice for now.
2. **VOICEVOX through the core** (`voicevox.rs`): `/speakers`, `/audio_query` with the speed set,
   `/synthesis`. Requests go from Rust, not the webview (no CORS, one place for errors). Every
   clip is kept on disk (`<data>/audio/voicevox/<style>/<hash>.wav`, keyed by text and speed),
   so a sentence is synthesized once (ADR 0016). The audio reaches the page as raw bytes and
   plays through Web Audio (exact end, pause by suspending).
3. **Reader first:** a Listen / Pause / Stop control and speed in the reader's toolbar, **P** to
   start and pause. The text is read sentence by sentence; the sentence being read is highlighted
   and scrolled into view. The word pop-up has 🔊 for the word (spoken from its kana reading, so
   a lone kanji isn't misread) and "Listen from here" for its sentence.
4. The reader shows the voice in use; for VOICEVOX as "VOICEVOX:<character>", as its terms ask.
5. **Next:** JLPT listening questions for practice tests (即時応答 and 課題理解 first), with
   AI-written scripts spoken by VOICEVOX speakers (several speakers for dialogues).

## Consequences

- Works offline on every desktop with a Japanese system voice; the quality of those voices
  varies (Windows' Japanese voices are installed under Speech settings). VOICEVOX sounds much
  better but is a separate install.
- The reader sends the text as written; the voice works out readings itself, so it can misread
  words Ankura's furigana gets right (and corrected readings aren't passed on yet).
- Audio files accumulate; nothing cleans them up yet (a sentence is about 100-300 KB).
