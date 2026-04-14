---
name: llm-wiki
description: Maintain an llm-wiki repository by using the llmwiki CLI for deterministic actions such as convert, prepare-ingest, lint, sync-state, and rebuild-index. Use when working inside a repository that contains AGENTS.md, raw/, wiki/, and state/.
---

# llm-wiki

Use this skill when the current repository follows the llm-wiki layout and the task is to maintain the knowledge workflow rather than write application code.

## What To Read First

1. Read `AGENTS.md` before changing repository content.
2. Read `wiki/_meta/index.md` before semantic query or synthesis work.
3. Use the `llmwiki` CLI for deterministic tasks instead of reimplementing filesystem logic in prompts.

## CLI Entry

- On Windows, run `scripts/llmwikiw.cmd`.
- On macOS or Linux, run `scripts/llmwikiw.sh`.

The wrapper resolves `LLMWIKI_BIN`, then the shared `llmwiki` install location, then the system `PATH`.
If no CLI is available, install it first by running `llmwiki install`.

## Preferred Command Usage

- `llmwiki convert <input>` for URL or file normalization into `raw/inbox/<slug>/`
- `llmwiki prepare-ingest <raw-path>` for agent ingest briefs
- `llmwiki lint` for repository health checks
- `llmwiki sync-state` to rebuild derived state
- `llmwiki rebuild-index` to rebuild `wiki/_meta/index.md`
- `llmwiki recent` and `llmwiki list` for inspection

## Guardrails

- Do not modify existing files under `raw/`.
- Preserve YAML frontmatter on wiki pages.
- Keep `wiki/_meta/index.md` and `wiki/_meta/log.md` synchronized after ingest, query, or lint work.
- Use `[[wiki links]]` for internal references.
