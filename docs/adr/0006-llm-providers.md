# 0006. LLM providers for v1: Anthropic and OpenRouter

- Status: accepted
- Date: 2026-10-26
- Deciders: owner, via AskUserQuestion in the phase 1 session

## Context

Ankura generates n+1 texts and example sentences, simplifies sentences and infers interests. The
user brings their own API key. HANDOFF §5 asks for current Claude models by default and a
deterministic check of the output.

## Decision

- **Anthropic:** native client. Defaults: `claude-sonnet-5-5` for text generation,
  `claude-haiku-5-5` for cheap tasks (simplify a sentence, extract interests). The model is a
  setting.
- **OpenRouter:** an OpenAI-compatible client against `https://openrouter.ai/api/v1`. Request JSON
  schema output with `provider.require_parameters: true`, so requests only go to endpoints that
  honour it.
- Ollama and other OpenAI-compatible endpoints are **not** in v1. The OpenAI-compatible client
  takes a base URL, so they can be added later with little work.
- Every generation goes through the deterministic check in `docs/architecture-options.md` §6:
  re-tokenize, compute coverage, check targets, retry at most twice, then gloss what's left.
- API keys are stored in the OS keychain on desktop and the Android Keystore on the phone, never in
  the sync batches.

## Consequences

- Two client implementations (Anthropic native, OpenAI-compatible) behind one interface.
- Structured output is validated on the client, because support varies between OpenRouter
  providers.
- Model IDs change over time. Keep them in configuration, with the defaults in one place.
