# 0011. AI texts without API costs: copy & paste, Claude Code, free OpenRouter models

- Status: accepted (owner, 2026-10-26); not yet built
- Date: 2026-10-26
- Deciders: owner

## Context

ADR 0010 calls the Anthropic API or OpenRouter with a paid key. The owner has no API key and
doesn't want to pay per text: they have a Claude Max (20x) subscription. That covers claude.ai and
Claude Code. API usage is billed separately.

## Decision

Offer three more ways to get a text. The existing API providers stay. Pick one in Settings ("How
Ankura talks to the AI"). When no API key is set, the default is copy & paste.

1. **Copy & paste via claude.ai (build first).**
   - Ankura shows the prompt with a Copy button. The prompt is the same one `generate.rs` builds,
     plus an instruction to answer with only the JSON object.
   - The user pastes the request into claude.ai and pastes Claude's reply back into a text box.
   - Ankura parses the reply leniently: strip ```` ```json ```` fences and take the first `{` to
     the last `}`. Then it runs the same coverage and target check as `generate.rs`.
   - Under 95%, or a target missing: Ankura shows the rewrite request (the same feedback text the
     retry loop uses) to copy. The user can also choose "Use it anyway".
   - To build this, split `generate_text` into `build_prompt`, `parse_answer`, `check` and
     `feedback`, so the automatic loop and the manual flow share them.
2. **Claude Code on the owner's PC.**
   - Ankura runs the user's installed, logged-in `claude` CLI in print mode: the prompt goes in on
     stdin, JSON comes out, and no tools are enabled. Usage counts against the Max plan.
   - Check the flags against `claude --help` before relying on them: `-p`,
     `--output-format json`, the structured-output or JSON-schema option, `--model`, and the
     options that disable tools.
   - On Windows the CLI may be `claude.exe` or `claude.cmd`. Ankura looks for it on PATH and
     shows a clear message if it can't find it.
   - Run it with the same retry loop as the API path.
   - Before shipping, check Anthropic's current terms on letting a personal app drive Claude Code
     with subscription login, and tell the owner what they say.
3. **Free OpenRouter models.**
   - The OpenRouter provider already exists. Add a picker of `:free` models, and say in the UI
     that free models write weaker Japanese and are rate limited.
   - `provider.require_parameters` with a JSON schema may leave no free provider. If so, fall back
     to the prompt-only JSON plus lenient parsing used in option 1.

## Consequences

- Copy & paste costs nothing extra and needs no setup, but every attempt takes manual steps.
- Claude Code is automatic, but the owner has to install and log in to Claude Code.
- The engine needs a JSON parser that tolerates fenced replies. Options 1 and 3 share it.
