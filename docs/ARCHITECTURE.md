# 系统架构

## 1. 目标

本项目旨在将零散来源整理为一个可长期维护、可审计、可持续演化的 Markdown Wiki。系统不是临时拼装答案的查询壳层，而是先沉淀知识，再围绕沉淀后的知识执行检索、问答、更新与校订。

## 2. 三层边界

### 2.1 `raw/`

`raw/` 是唯一事实输入层，用于保存原始资料与转换产物。

- `raw/inbox/`：待处理来源与 `convert` 输出 bundle
- `raw/sources/`：已登记原件
- `raw/assets/`：共享原始资源

该层既有内容不得被 agent 直接改写。

### 2.2 `wiki/`

`wiki/` 是知识沉淀层，用于保存来源总结、实体、概念、问题、综合与时间线页面。

- `wiki/sources/`
- `wiki/entities/`
- `wiki/concepts/`
- `wiki/questions/`
- `wiki/syntheses/`
- `wiki/timelines/`
- `wiki/_meta/index.md`
- `wiki/_meta/log.md`

### 2.3 `AGENTS.md`

`AGENTS.md` 是行为规范层，定义：

- 页面命名
- frontmatter 契约
- ingest/query/lint 工作流
- 链接、引用与冲突处理规范
- 索引与日志的维护义务

## 3. CLI 的职责

Rust CLI 是确定性执行面，负责：

- 仓库初始化
- URL / 文件到 Markdown bundle 的转换
- 索引重建
- 派生状态重建
- 机械 lint
- sidecar 安装
- 共享 CLI 安装
- harness skill 安装与诊断

CLI 不负责：

- 语义检索
- 跨页综合
- 问题回答
- 冲突裁决
- 高价值结论的最终写回

这些工作仍由 agent 按 `AGENTS.md` 执行。

## 4. 转换子系统

`convert` 将支持的 URL 或本地文件统一落为：

```text
raw/inbox/<slug>/
  note.md
  metadata.json
  assets/
  source/
```

当前覆盖范围：

- 普通网页
- 公众号文章
- 知乎页面
- B 站视频页
- 抖音视频页
- PDF、DOCX、PPTX、XLSX
- XLS、XLSM、XLSB、XLA、XLAM、ODS
- Markdown、HTML、TXT、JSON、XML

视频页通过 `yt-dlp` sidecar 处理；其解析顺序为：

1. `LLMWIKI_YT_DLP`
2. 仓库内 `tools/yt-dlp/<platform>/`
3. 系统 `PATH`

## 5. 共享 CLI 与 Skill 层

### 5.1 共享 CLI

`llmwiki install` 将当前运行中的二进制复制到共享路径，供多个 harness 复用。

- Windows：`%LOCALAPPDATA%\\llmwiki\\bin\\llmwiki.exe`
- macOS / Linux：`${XDG_DATA_HOME:-~/.local/share}/llmwiki/bin/llmwiki`

这条共享路径是所有 wrapper 的统一调用目标。

### 5.2 Skill 层

skill 是 harness 发现层，而不是第二套业务逻辑。其职责仅包括：

- 向 harness 暴露 `llm-wiki` 工作流入口
- 引导 agent 读取 `AGENTS.md`
- 通过轻量 wrapper 调用共享 CLI

skill 本体不内嵌 `llmwiki` 二进制。

### 5.3 安装目标

repo 级安装目标：

- Claude：`.claude/skills/llm-wiki`
- OpenCode：`.opencode/skills/llm-wiki`
- OpenClaw：`skills/llm-wiki`
- Codex：`.agents/skills/llm-wiki`

user 级安装目标：

- Claude：`~/.claude/skills/llm-wiki`
- OpenCode：`~/.config/opencode/skills/llm-wiki`
- OpenClaw：`~/.openclaw/skills/llm-wiki`
- Codex：`$HOME/.agents/skills/llm-wiki`

### 5.4 诊断

`llmwiki skill doctor` 用于检查：

- 共享 CLI 是否存在
- skill 目录是否存在
- `SKILL.md` 是否存在
- wrapper 脚本是否完整

`llmwiki doctor` 则保留给 `convert` 与 sidecar 相关诊断。

## 6. 派生状态

`state/` 只保存可重建内容，例如：

- `source_manifest.jsonl`
- `page_graph.json`
- `lint-latest.json`

`state/` 不得成为事实源，只能作为加速层与诊断层。

## 7. 运行原则

### 7.1 写入原则

- 原始资料只进 `raw/`
- 长期知识只进 `wiki/`
- 自动化与缓存只进 `state/`

### 7.2 可追溯性优先

当实现选择存在分歧时，优先级为：

1. 可追溯性
2. 可维护性
3. 可读性
4. 可扩展性
5. 美观性

### 7.3 中立模板

`init` 默认不安装任何 harness skill，只在用户显式传入 `--install-skill` 时进行附加安装。这样可以保持 starter 仓库模板的中立性。
