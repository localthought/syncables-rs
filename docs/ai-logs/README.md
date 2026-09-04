# Generative AI prompt/output logs

This folder is syncables-rs' disclosure log for generative-AI use, kept to
comply with [NLnet's Generative AI policy](https://nlnet.nl/foundation/policies/generativeAI/)
for NLnet-funded work. It mirrors the structure used in
[localthought/syncables](https://github.com/localthought/syncables/tree/main/docs/ai-logs),
the TypeScript project this crate is a port of.

## What's logged here, and what isn't

syncables-rs has been developed collaboratively with Claude Code
(Anthropic), an agentic coding assistant, from its first commit. Unlike the
TypeScript original, this repo has no pre-policy history, so there is
nothing to backfill: every session that produces a commit gets a log under
[`sessions/`](sessions), redacted per the rules below.

- Session logs capture the **substantive human prompts and the assistant's
  substantive outputs** — the actual asks and the actual answers/code
  changes. They do not reproduce the coding assistant's internal system
  prompt, tool-call plumbing, or other harness scaffolding verbatim: that
  content is Anthropic product internals rather than project-specific
  "prompts," and dumping it wouldn't add transparency about how *this
  project* was built.
- Commits produced with AI assistance also carry a
  `Claude-Session: https://claude.ai/code/session_...` trailer, so a commit
  can be traced to the log entry for the session that produced it.

## Redaction

Before anything is committed here, logs are reviewed and redacted for:

- credentials, tokens, and anything else that looks like a secret;
- personal information (e.g. email addresses) not otherwise already public
  about the project;
- any other session- or account-identifying detail that isn't needed to
  understand what was asked and what was produced.

Redacted spans are marked `[redacted]` inline rather than silently deleted,
so it's visible that a redaction happened.

## See also

- The [README's "Generative AI use" section](../../README.md#generative-ai-use)
  for the project-level summary this policy asks for.
- [NLnet's Generative AI policy](https://nlnet.nl/foundation/policies/generativeAI/).
