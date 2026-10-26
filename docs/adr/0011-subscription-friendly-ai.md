# 0011. AI texts without API costs: copy & paste, Claude Code, free OpenRouter models

- Status: accepted (owner, 2026-10-26); option 1 built, options 2-3 on hold (see Update)
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

## Update, 2026-10-26: Max plans include API credits

Anthropic now gives Claude Max subscribers monthly API credits ($100 on Max 5x, $200 on Max 20x;
expire each billing month, claimed by linking a Claude Console organization to the plan). They
cover the Claude API, so the existing Anthropic provider costs the owner nothing within that
budget. Sources: <https://support.claude.com/en/articles/17154008>,
<https://platform.claude.com/docs/en/about-claude/api-credits-for-subscribers>.

The owner chose: claim the credits and use the API key, and build copy & paste (option 1) as the
no-setup fallback. Options 2 (Claude Code CLI) and 3 (free OpenRouter models) are on hold.

Notes for option 2 if it comes back:

- Checked with Claude Code 2.1.310: `claude -p --output-format json --json-schema '<schema>'
  --tools "" --system-prompt '<system>' --model <model> --no-session-persistence
  --setting-sources "" --strict-mcp-config` with the prompt on stdin works. The answer is in the
  result's `structured_output` field; `is_error` and `result` carry failures.
- Terms (support.claude.com article 15036540, updated 2026-10-07): `claude -p`, the Agent SDK and
  third-party apps still draw from subscription limits; Anthropic paused a planned change to that
  and says it will announce updates first.

As built, option 1 is a mode, not a provider: Settings and the Write screen switch between
"Automatically, with an API key" and "Copy & paste through claude.ai" (setting `ai_mode`; the
default is copy & paste when no key is saved). The rewrite request for the same chat is only the
feedback plus "answer with the JSON again", not the whole prompt. A reply with no JSON is read as
plain text, with a `# Title` first line as the title.
