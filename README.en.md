# llm-wiki

[Landing page](./README.md) | [中文说明](./README.zh-CN.md)

`llm-wiki` is a local-first, Markdown-first, Git-backed knowledge workspace maintained over time by agents. It is not a thin RAG shell over raw files. The intended model is to turn sources into a durable wiki first, then retrieve, answer, revise, and extend knowledge from that wiki.

## Project Scope

This repository ships two things:

- a starter repository layout for a long-lived Markdown wiki
- a provider-neutral Rust CLI for deterministic maintenance tasks

The CLI is intentionally narrow. It handles initialization, conversion, indexing, state rebuilding, sidecar installation, shared CLI installation, and harness skill installation. Semantic interpretation, cross-page synthesis, conflict handling, and long-form knowledge writing remain the agent's job under [AGENTS.md](./AGENTS.md).

## Three-Layer Model

The repository follows a strict three-layer structure:

1. `raw/`
   Immutable source inputs and conversion outputs. Existing material in this layer should not be edited by agents.
2. `wiki/`
   Curated knowledge pages including source summaries, entities, concepts, questions, syntheses, and timelines.
3. `AGENTS.md`
   The behavioral contract for ingest, query, lint, naming, frontmatter, linking, and logging.

This separation is the core design decision of the project.

## Repository Layout

High-level layout:

- `raw/`
  - `raw/inbox/`: incoming material and `convert` bundles
  - `raw/sources/`: registered originals
  - `raw/assets/`: shared raw assets
- `wiki/`
  - `wiki/sources/`
  - `wiki/entities/`
  - `wiki/concepts/`
  - `wiki/questions/`
  - `wiki/syntheses/`
  - `wiki/timelines/`
  - `wiki/_meta/index.md`
  - `wiki/_meta/log.md`
- `state/`
  - rebuildable derived state such as manifests, page graphs, and lint output
- `templates/`
  - page templates
- `docs/`
  - architecture notes, execution plans, and technical plans

## CLI Commands

Current commands:

- `llmwiki init [--install-skill <harness>]`
  Initialize a repository with the starter layout. No harness skill is installed by default.
- `llmwiki install [--force]`
  Install the currently running `llmwiki` binary into a shared location.
- `llmwiki skill install --harness <claude|opencode|openclaw|codex> [--scope <repo|user>]`
  Install a thin harness-specific skill and ensure the shared CLI is available.
- `llmwiki skill doctor [--harness <...>] [--scope <repo|user>]`
  Diagnose the shared CLI plus required skill files for a harness and scope.
- `llmwiki convert <input>`
  Convert a supported URL or local file into a normalized Markdown bundle.
- `llmwiki doctor`
  Check conversion prerequisites, output directories, and optional sidecars.
- `llmwiki install-sidecar yt-dlp`
  Install a repo-local `yt-dlp` binary for Bilibili and Douyin video conversion.
- `llmwiki prepare-ingest <raw-path>`
  Generate a stable ingest brief for an external agent.
- `llmwiki lint`
  Produce structural and semantic health signals.
- `llmwiki rebuild-index`
  Rebuild `wiki/_meta/index.md`.
- `llmwiki sync-state`
  Rebuild derived files under `state/`.
- `llmwiki recent`
  Show recent log entries.
- `llmwiki list`
  List pages by type.

Two command-level clarifications matter:

- `prepare-ingest` does not execute the full ingest workflow by itself; it prepares the agent-facing brief.
- `ask` no longer exists; query answering belongs to the agent workflow defined in `AGENTS.md`.

## Shared CLI and Skills

`llmwiki install` copies the current binary into a shared location so multiple harness skills can reuse one installation.

- Windows: `%LOCALAPPDATA%\\llmwiki\\bin\\llmwiki.exe`
- macOS / Linux: `${XDG_DATA_HOME:-~/.local/share}/llmwiki/bin/llmwiki`

`llmwiki skill install` is a composite action:

1. ensure the shared CLI is installed
2. write the harness-specific skill directory
3. install thin wrapper scripts that call the shared CLI

Repo-scope skill targets:

- Claude: `.claude/skills/llm-wiki`
- OpenCode: `.opencode/skills/llm-wiki`
- OpenClaw: `skills/llm-wiki`
- Codex: `.agents/skills/llm-wiki`

User-scope targets:

- Claude: `~/.claude/skills/llm-wiki`
- OpenCode: `~/.config/opencode/skills/llm-wiki`
- OpenClaw: `~/.openclaw/skills/llm-wiki`
- Codex: `$HOME/.agents/skills/llm-wiki`

Typical commands:

```powershell
llmwiki install
llmwiki skill install --harness codex --scope repo
llmwiki skill doctor --harness codex --scope repo
```

## What `convert` Produces

`llmwiki convert` writes a normalized bundle to `raw/inbox/<slug>/`:

```text
<slug>/
  note.md
  metadata.json
  assets/
  source/
```

Bundle contents:

- `note.md`
  The normalized Markdown body intended for downstream ingest.
- `metadata.json`
  Source type, platform, converter chain, capture time, warnings, and asset inventory.
- `assets/`
  Downloaded images, thumbnails, subtitles, and similar local resources.
- `source/`
  Original HTML, copied source files, video metadata, and sidecar outputs.

Supported inputs today:

- generic web URLs such as article pages, blogs, and documentation pages
- WeChat articles and Zhihu pages
- Bilibili and Douyin video pages through `yt-dlp`
- PDF, DOCX, PPTX, XLSX
- XLS, XLSM, XLSB, XLA, XLAM, ODS
- Markdown, HTML, TXT, JSON, XML

## Conversion Boundaries

The current conversion strategy is intentionally narrow:

- generic web pages and Zhihu use the Rust extraction chain
- WeChat articles use a Rust adapter first, with `wechat-article-to-markdown` as an optional fallback
- Bilibili and Douyin use `yt-dlp`
- documents and spreadsheets use Rust-native libraries wherever practical

For video pages, `yt-dlp` is resolved in this order:

1. `LLMWIKI_YT_DLP`
2. `tools/yt-dlp/<platform>/` inside the repository
3. system `PATH`

If you want a repo-local sidecar:

```powershell
llmwiki install-sidecar yt-dlp
```

## Recommended Workflow

A practical day-to-day workflow looks like this:

1. Use `llmwiki convert` to turn a URL or file into a bundle under `raw/inbox/`
2. Verify that the bundle is complete
3. Run `llmwiki prepare-ingest ...` to generate the ingest brief
4. Hand the brief to the agent so it can update source pages, entity pages, concept pages, index, and log
5. Run `llmwiki lint` periodically
6. Persist high-value answers into `wiki/questions/` or `wiki/syntheses/`

## Non-Negotiable Rules

The authoritative contract is [AGENTS.md](./AGENTS.md), but the most important rules are:

- do not modify existing material under `raw/`
- do not write deterministic claims without source support
- after ingest, query, or lint, keep `wiki/_meta/index.md` and `wiki/_meta/log.md` in sync
- every wiki page must retain YAML frontmatter
- use `[[wiki link]]` consistently across pages
- conflicts must be made explicit rather than silently merged away

## Current State

The repository already supports:

- starter repository initialization
- index and derived-state rebuilding
- mechanical linting and log updates
- conversion of multiple URL and file types into a unified Markdown bundle
- shared CLI installation and multi-harness skill installation
- diagnosis of shared CLI and skill completeness
- repo-local `yt-dlp` sidecar installation for video conversion

The agent still owns:

- semantic search
- cross-page synthesis
- conflict interpretation
- long-form answer writing
- ongoing knowledge evolution

## Related Documents

- [Landing page](./README.md)
- [中文说明](./README.zh-CN.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [Agent rules](./AGENTS.md)
- [Execution plan](./docs/EXECUTION_PLAN_zh.md)
