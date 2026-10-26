# Ankura: handoff to the first working agent

> **Historical.** Phase 1 below is done (research docs, ADRs 0001-0010, M1, most of M2). The
> vision and requirements still apply; for the current state read `docs/NEXT-SESSION.md`.

Date written: 2026-10-26. Written by the setup session, which deliberately did **not** start
building. You are the first agent to do real work here.

## 1. What the owner wants (their words, condensed)

> Do deep research on open-source language learning tools like Lute (https://github.com/LuteOrg)
> and Anki and find more such projects. I want my own tool that combines the best of them or
> integrates them (I have used Anki for nearly 20 years). It should be fun and easy to use and
> work seamlessly as a desktop app and as an Android app.

Functional requirements:

1. **Text input:** enter or paste texts and read them in the tool.
2. **Known-word display:** show which words, kanji, and phrases the user already knows or is
   *supposed* to know.
3. **Anki awareness:** read the user's Anki card stats to decide what they're supposed to know.
4. **Own telemetry:** track how often each word, kanji, or phrase has been *seen* in the tool and
   how often the user clicked "I don't know this".
5. **AI generation:** given a list of new words or kanji, generate one or more texts plus sample
   sentences (for example for Anki cards) at several difficulty levels. Goal: mostly known words,
   few unknown ones (n+1 / comprehensible input).
6. **Interest profile:** infer rough interests from the texts the user pastes, offer a screen to
   edit them manually, and use them as the main steer for generated texts.
7. **More ideas welcome:** suggest features that fit, especially ones existing tools already
   have.

## 2. What already exists

- GitHub repo `drkthng/ankura` (public, MIT). Repo description: *"Cross-platform extensive
  reading tool (Desktop & Android) synced with Anki. Ankura tracks vocabulary exposure and lookup
  friction, using AI to generate interest-tailored n+1 texts with high known-word density."*
- `docs/research/prior-ai-report.md`: a research and architecture report from a less capable
  model. It is a useful starting map, but **every factual claim in it has to be checked**.
- No code, no stack decision.

## 3. Your first phase: research verification and architecture proposal (no app code)

The deliverables, all in one draft PR:

1. **`docs/research/landscape.md`**: a verified overview of relevant open-source (and notable
   closed) tools. For each one: repo URL, license, maintenance status (date of last commit or
   release, archived or not), tech stack, platforms, how it talks to Anki, and what Ankura should
   borrow, integrate with, or avoid. Use web search and read the actual repos and docs.
2. **`docs/research/claims-check.md`**: go through the prior report's claims (list in §4) and mark
   each one confirmed, wrong (with the correction), or unverifiable, citing sources.
3. **`docs/architecture-options.md`**: 2-3 viable architectures with trade-offs and a
   recommendation. Must answer §5's hard questions.
4. **Open decisions:** put the decisions in §6 to the owner with `AskUserQuestion` (after the
   research, so the options are informed). Record the answers in `docs/adr/`.
5. Optional, after the owner decides: a phased roadmap (`docs/ROADMAP.md`) with a thin first
   vertical slice, then a short summary of the next step.

Don't scaffold the app until the owner has approved the architecture.

## 4. Claims from the prior report that need checking

The setup agent already had doubts about the items marked ⚠.

| # | Claim | Note |
|---|-------|------|
| 1 | Lute v3 lives at `jzohrab/lute-v3`; Python/Flask/SQLite; parsers MeCab, Jieba, spaCy | ⚠ The owner links `LuteOrg`; the repo may have moved. Check whether Lute now has Anki integration and any mobile story. |
| 2 | LWT community fork `HugoFara/lwt` has a Kotlin Android client called "Lukaisu" | ⚠ Sounds invented. Verify it exists. |
| 3 | AnkiMorphs (`mortii/anki-morphs`) tracks known morphs by interval and scores n+1 | Probably right; check how it defines "known" and whether its logic can be reused. |
| 4 | Jidoujisho (`lrorpilla/jidoujisho`) is a Flutter Android app using AnkiDroid's API | ⚠ It may be discontinued or archived. Check status and any successor. |
| 5 | read-bridge (`WindChimeEcho/read-bridge`) does LLM-driven n+1 reading | Verify that it exists and what it does. |
| 6 | AnkiDroid ContentProvider URI is `content://com.ichi2.anki.provider/cards` | ⚠ The setup agent believes the authority is `com.ichi2.anki.flashcards`. **Critical:** check whether the provider exposes interval, ease, lapses, and FSRS state at all. If it doesn't, the Android plan has to change. |
| 7 | AnkiConnect `cardsInfo` returns interval, reps, lapses, factor, and FSRS stability and difficulty | Check FSRS fields specifically (Anki 23.10+ stores a memory state). |
| 8 | "Known" means interval ≥ 21d or stability ≥ 30 | Arbitrary. Consider FSRS retrievability (probability of recall today) as a better signal. |
| 9 | Tauri v2 targets Android with a Kotlin plugin bridge; Lindera runs there | Check maturity, APK size with dictionaries, and the plugin story for ContentResolver. |
| 10 | Never read `collection.anki2` while Anki runs | Broadly right. Check whether reading a *copy* or a backup is safe and useful. |
| 11 | Android 14+ kills long background work | Generic. Only matters if heavy work runs on-device. |

## 5. Hard questions the architecture has to answer

- **Android Anki data:** AnkiConnect doesn't exist on Android. If the AnkiDroid provider lacks
  scheduling stats, alternatives include: (a) compute a "knowledge snapshot" on desktop through
  AnkiConnect and sync it to the phone through Ankura's own sync; (b) use Anki's open-source
  self-hosted sync server or protocol; (c) import `.apkg`/`.colpkg` backups. Evaluate these.
- **Note → lemma mapping:** a 20-year collection has many note types and messy fields (HTML,
  furigana markup, inflected forms). How does the user configure which field holds the target
  word, and how are fields normalized to lemmas and kanji?
- **Build vs integrate:** fork or extend Lute, build on LinguaCafe, or start fresh? Make a
  genuine case for each.
- **Sync between desktop and phone** for Ankura's own data (telemetry, texts, interests):
  self-hosted, file-based (Syncthing or a cloud folder), or a CRDT library?
- **LLM provider:** the user can bring their own key (Anthropic, OpenRouter, or local Ollama).
  Default to current Claude models. Generated texts need a deterministic check: re-tokenize and
  compute the known-word ratio, then regenerate or simplify if it's too low.
- **Language scope:** Japanese first (kanji tracking), but should the design be
  language-agnostic from day one?

## 6. Decisions to put to the owner (via AskUserQuestion, after research)

- Target language(s) for v1.
- Build vs fork/integrate (with your recommendation).
- Stack (for example Tauri v2 + Svelte/React, Flutter, or Kotlin Multiplatform).
- How the phone gets Anki stats (see §5).
- Sync approach for Ankura's own data.
- LLM providers to support first.
- Whether Ankura may *write* to Anki (create cards) in v1 or only read.

## 7. Tools to look at beyond the prior report (setup agent's suggestions, all unverified)

Open source: LinguaCafe (self-hosted reading, Japanese support), Yomitan, VocabSieve, asbplayer,
Mokuro, ttu ebook reader, Memento, KOReader plus its Anki plugin, FSRS
(open-spaced-repetition), the Anki sync server, MorphMan (the predecessor of AnkiMorphs).
Closed, but worth learning from: LingQ, Migaku, jpdb.io, Satori Reader, Readlang, Language
Reactor.

## 8. Feature ideas from the prior report worth evaluating

- Comprehensibility preview before reading a text (bar of known/learning/unknown shares).
- Kanji popover listing the other words in the user's deck that use the same kanji.
- Exposure counts only once a paragraph has been in the viewport for a few seconds.
- "Friction ratio" (lookups divided by exposures) to catch "stealth leeches": words Anki says
  are mature but that keep tripping up reading.
- One-click context card to Anki (sentence, gloss, reading, TTS audio).
- "Simplify this sentence" button that rewrites the sentence using known vocabulary.
- Weaving the user's Anki leeches into generated texts as supporting context.
- Interest profile with time-decayed weights plus manual overrides (a tag cloud with weights and
  custom scenario prompts).
