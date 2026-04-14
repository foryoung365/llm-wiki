# llm-wiki

[入口页](./README.md) | [English](./README.en.md)

`llm-wiki` 是一个本地优先、Markdown 优先、由 Git 承载并由 agent 持续维护的知识工作区。它不是把原始资料直接挂到临时问答链路上的 RAG 壳层，而是先将来源整理为可维护的 Wiki，再围绕该 Wiki 进行检索、回答、校订与持续沉淀。

## 项目定位

本仓库同时提供两类内容：

- 一套可直接启用的 LLM Wiki 仓库骨架
- 一个 provider-neutral Rust CLI，用于执行可重复、可审计、可脚本化的维护动作

命令行工具的职责边界是刻意收窄的：它负责目录初始化、转换、索引重建、状态同步、日志辅助、共享 CLI 安装与 harness skill 安装；语义理解、跨页综合、冲突解释与高价值知识落盘，仍由 agent 按 [AGENTS.md](./AGENTS.md) 执行。

## 三层模型

系统严格遵循三层分工：

1. `raw/`
   原始资料层。保存原始输入与转换产物；既有资料不得被 agent 直接改写。
2. `wiki/`
   知识沉淀层。保存来源总结、实体页、概念页、问题页、综合页与时间线。
3. `AGENTS.md`
   行为规范层。定义命名、frontmatter、引用、日志与 ingest/query/lint 工作流。

这条边界是本项目最重要的设计前提。

## 目录结构

仓库高层结构如下：

- `raw/`
  - `raw/inbox/`：待处理来源与 `convert` 输出 bundle
  - `raw/sources/`：已登记原件
  - `raw/assets/`：共享原始资源
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
  - 可重建的派生状态，例如 manifest、page graph 与 lint 结果
- `templates/`
  - 页面模板
- `docs/`
  - 架构说明、执行计划与实施计划

## CLI 命令

当前公开命令如下：

- `llmwiki init [--install-skill <harness>]`
  初始化仓库骨架；默认不安装任何 harness skill。若显式追加 `--install-skill`，会在初始化完成后立即安装对应 skill。
- `llmwiki install [--force]`
  将当前运行中的 `llmwiki` 安装到共享路径，供多个 harness skill 复用。
- `llmwiki skill install --harness <claude|opencode|openclaw|codex> [--scope <repo|user>]`
  安装指定 harness 的薄 skill，并确保共享 CLI 已就绪。
- `llmwiki skill doctor [--harness <...>] [--scope <repo|user>]`
  诊断共享 CLI 与 skill 目录是否完整，指出缺失的 `SKILL.md`、wrapper 脚本或错误目标路径。
- `llmwiki convert <input>`
  将支持的 URL 或本地文件转换为标准 Markdown bundle。
- `llmwiki doctor`
  检查 `convert` 相关依赖、输出目录与 sidecar 可用性。
- `llmwiki install-sidecar yt-dlp`
  安装仓库内 `yt-dlp` sidecar，用于 B 站与抖音视频页转换。
- `llmwiki prepare-ingest <raw-path>`
  为 agent 生成 ingest brief；并不直接执行完整 ingest。
- `llmwiki lint`
  生成结构与语义健康检查结果。
- `llmwiki rebuild-index`
  重建 `wiki/_meta/index.md`。
- `llmwiki sync-state`
  重建 `state/` 下的派生状态。
- `llmwiki recent`
  查看最近日志条目。
- `llmwiki list`
  按页面类型列出条目。

需要特别说明两点：

- `prepare-ingest` 是 agent 工作流的准备入口，不是自动 ingest 执行器。
- `ask` 已移除；语义检索与问答写回应由 agent 按 `AGENTS.md` 的 Query 工作流完成。

## 共享 CLI 与 Skill 安装

`llmwiki install` 会把当前正在运行的二进制复制到共享位置，供不同 harness 的 skill 统一调用。

- Windows：`%LOCALAPPDATA%\\llmwiki\\bin\\llmwiki.exe`
- macOS / Linux：`${XDG_DATA_HOME:-~/.local/share}/llmwiki/bin/llmwiki`

`llmwiki skill install` 的职责是组合式的：

1. 确保共享 CLI 已安装
2. 写入对应 harness 的 skill 目录
3. 安装用于调用共享 CLI 的轻量 wrapper 脚本

当前 repo 级 skill 目录如下：

- Claude：`.claude/skills/llm-wiki`
- OpenCode：`.opencode/skills/llm-wiki`
- OpenClaw：`skills/llm-wiki`
- Codex：`.agents/skills/llm-wiki`

用户级目录如下：

- Claude：`~/.claude/skills/llm-wiki`
- OpenCode：`~/.config/opencode/skills/llm-wiki`
- OpenClaw：`~/.openclaw/skills/llm-wiki`
- Codex：`$HOME/.agents/skills/llm-wiki`

常见示例：

```powershell
llmwiki install
llmwiki skill install --harness codex --scope repo
llmwiki skill doctor --harness codex --scope repo
```

## convert 的输出契约

`llmwiki convert` 会把输入统一写入 `raw/inbox/<slug>/` 下的一组 bundle：

```text
<slug>/
  note.md
  metadata.json
  assets/
  source/
```

各文件职责如下：

- `note.md`
  统一后的 Markdown 正文，可直接进入后续 ingest 流程。
- `metadata.json`
  记录来源类型、平台、转换链、抓取时间、警告与资源清单。
- `assets/`
  保存图片、缩略图、字幕等本地化资源。
- `source/`
  保存原始 HTML、原始文件副本、视频元数据与 sidecar 输出。

当前支持的输入包括：

- 普通网页 URL：新闻、博客、文档页、文章页
- 社媒 URL：公众号文章、知乎页、B 站视频页、抖音视频页
- 文档输入：PDF、DOCX、PPTX、XLSX
- 表格家族：XLS、XLSM、XLSB、XLA、XLAM、ODS
- 文本类文件：Markdown、HTML、TXT、JSON、XML

## 转换边界

当前转换链采取“Rust 主体 + 极少量 sidecar”的策略：

- 普通网页与知乎页：走 Rust 主抽取链
- 公众号文章：默认先走 Rust 适配器；必要时回退到 `wechat-article-to-markdown`
- B 站与抖音视频页：通过 `yt-dlp` sidecar 抽取元数据、缩略图与字幕
- 文档与表格：尽量使用纯 Rust 库完成解析与 Markdown 收敛

视频页使用的 `yt-dlp` 按以下顺序解析：

1. `LLMWIKI_YT_DLP`
2. 仓库内 `tools/yt-dlp/<platform>/`
3. 系统 `PATH`

如需安装仓库内 sidecar，可执行：

```powershell
llmwiki install-sidecar yt-dlp
```

## 推荐工作流

一个清晰且最小化的日常流程如下：

1. 使用 `llmwiki convert` 将 URL 或文件转换到 `raw/inbox/<slug>/`
2. 检查 bundle 是否完整
3. 运行 `llmwiki prepare-ingest ...` 生成 ingest brief
4. 将 brief 交由 agent，按 `AGENTS.md` 更新来源页、实体页、概念页、索引与日志
5. 周期性执行 `llmwiki lint`
6. 对高价值问题，要求 agent 将结论沉淀到 `wiki/questions/` 或 `wiki/syntheses/`

## 关键规则

权威规则以 [AGENTS.md](./AGENTS.md) 为准，但至少应牢记以下几点：

- 不得修改 `raw/` 中任何既有资料
- 不得在缺少来源支持时写入确定性断言
- 每次 ingest / query / lint 后，都要同步维护 `wiki/_meta/index.md` 与 `wiki/_meta/log.md`
- 所有 Wiki 页面都必须保留 YAML frontmatter
- 页面之间统一使用 `[[wiki link]]`
- 遇到冲突信息时必须显式并置来源，不得私自抹平

## 当前状态

当前仓库已经具备以下能力：

- 初始化合规的 LLM Wiki starter 仓库
- 重建索引与派生状态
- 执行机械 lint 与日志追加
- 将多类 URL / 文件转换为统一 Markdown bundle
- 安装共享 CLI 与多 harness skill
- 诊断共享 CLI 与 skill 目录完整性
- 通过 repo-local `yt-dlp` sidecar 支持视频页转换

仍由 agent 主导的部分包括：

- 语义检索
- 跨页综合
- 冲突解释
- 高价值问答落盘
- 长期知识结构演化

## 相关文档

- [入口页](./README.md)
- [English version](./README.en.md)
- [系统架构](./docs/ARCHITECTURE.md)
- [Agent 规则](./AGENTS.md)
- [执行计划](./docs/EXECUTION_PLAN_zh.md)
