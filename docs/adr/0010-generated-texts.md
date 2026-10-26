# 0010. Generated texts: providers, defaults, and the coverage check

- Status: accepted
- Date: 2026-10-26
- Deciders: agent, within ADR 0006 (owner chose Anthropic and OpenRouter); owner to review

## Context

ROADMAP M2: Ankura writes reading texts around words the user is learning, made mostly of words
the user knows. ADR 0006 chose the providers (Anthropic, OpenRouter) and proposed Sonnet as the
default model. Ankura's engine is Rust; Anthropic has no official Rust SDK.

## Decision

- **Calls:** raw HTTPS from `crates/ankura-core/src/llm.rs`.
  - Anthropic: `POST /v1/messages`, `anthropic-version: 2026-10-21`, structured output through
    `output_config.format` (JSON schema).
  - OpenRouter: chat completions with `response_format` `json_schema`, and
    `provider.require_parameters: true` so only providers that honour the schema are used.
  - Both are behind one `Llm` trait. Ankura parses the answer against its own types.
- **Default model `claude-opus-5-5` at effort `medium`.** This replaces ADR 0006's Sonnet default,
  following current Anthropic guidance to use the current Opus unless the user picks another.
  Model and effort are settings. A short text costs a few cents.
- **Refusal fallback:** Anthropic requests send `fallbacks: "default"` (beta
  `server-side-fallback-2026-07-01`). If a safety classifier declines, Anthropic re-runs the
  request on its recommended fallback model. It's unlikely to matter for reading texts and can
  be removed if the owner prefers.
- **No sampling parameters** (`temperature` etc.); API version 2026-10-21 rejects them.
- **API keys** in the OS keychain (keyring crate). `ANKURA_API_KEY` and `ANKURA_LLM_URL` exist only
  for development and tests (`tools/dev/fake_llm.py`).
- **Pipeline** (`generate.rs`, architecture-options §6):
  1. The prompt has the target words (as the user sees them), the topic, length and style, up to
     8 fading words to weave in, and the 400 most stable known words.
  2. The answer has a title, paragraphs, and one example sentence with a translation per target.
  3. Ankura re-tokenizes the text. Coverage counts known words, basic words and targets.
  4. If coverage is under 95% or a target is missing, Ankura asks for a rewrite, naming the
     unknown words and missing targets. At most 3 attempts; the best one is kept.
  5. The text is saved to the library with its targets and examples.
- **Generation uses its own database connection**, so the app stays responsive while the model
  writes (SQLite WAL, 10 s busy timeout).
- **Basic words** (する, ある, いる, こと, この…) count as known unless the user marks them, in the
  reader and in the coverage check, matching what the prompt allows the model to use.

## Consequences

- The owner needs an Anthropic (or OpenRouter) API key and pays per text.
- Prompt caching isn't used: the prompt is short and changes with each request.
- The coverage threshold and the attempt limit are constants for now; make them settings if
  texts come out too easy or too hard.
