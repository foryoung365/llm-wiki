# 纯 Karpathy 内核版 llm-wiki（Rust）实施计划

## 1. 目标边界

本版本只实现 Karpathy 原文中的最小模式：

- `raw/`：不可变原始资料层
- `wiki/`：由外部 LLM agent 维护的 Markdown 知识层
- `AGENTS.md`：约束 agent 工作方式的 schema/说明文件
- `index.md`：全库目录与摘要
- `log.md`：按时间追加的操作日志
- 三类工作流：`ingest`、`query`、`lint`

Rust CLI **不接任何 LLM provider**，也**不内置 autonomous ingest/query**。它只做确定性辅助：初始化、扫描、重建、提示词生成、健康检查。

## 2. 不做的内容

以下内容全部排除在纯内核版之外：

- 内置 OpenAI / Anthropic / 本地模型 provider
- 向量库、RAG、BM25、状态数据库
- JSON Schema 公共 API
- 严格 frontmatter 契约
- 平台绑定的 GitHub / GitLab / Gitea API 流程
- hooks、CODEOWNERS、CI gate 作为架构前提
- 自动生成 slide/chart/canvas 等派生产物

## 3. 最小目录结构

```text
repo/
├─ raw/
│  ├─ inbox/
│  └─ assets/
├─ wiki/
│  ├─ sources/
│  ├─ entities/
│  ├─ concepts/
│  └─ syntheses/
├─ AGENTS.md
├─ index.md
└─ log.md
```

说明：

- `raw/` 只增不改，除人工校正文件名外不做内容修改。
- `wiki/` 全部由外部 agent 读写。
- `index.md` 与 `log.md` 位于仓库顶层，便于 agent 首读。
- `wiki/` 子目录只是约定俗成，不是强制 schema。

## 4. Rust CLI 角色

CLI 只做五件事：

1. `init`：生成最小骨架。
2. `index rebuild`：根据现有 `wiki/` 页面重建 `index.md`。
3. `log append`：按统一格式追加 `log.md` 条目。
4. `lint`：做纯机械健康检查。
5. `prompt`：为外部 agent 生成 `ingest/query/lint` 工作提示。

因此，这不是“用 Rust 替代 agent”，而是“用 Rust 把仓库维护动作做成稳定的外部工具”。

## 5. CLI 命令面

```text
llmwiki init
llmwiki index rebuild
llmwiki log append --kind ingest --title "..."
llmwiki lint
llmwiki prompt ingest <raw-path>
llmwiki prompt query "<question>"
llmwiki prompt lint
llmwiki recent
llmwiki list
```

### 5.1 `llmwiki init`

作用：

- 创建 `raw/`、`wiki/`、`AGENTS.md`、`index.md`、`log.md`
- 写入最小模板

### 5.2 `llmwiki index rebuild`

作用：

- 扫描 `wiki/**/*.md`
- 提取标题与第一页有效摘要
- 生成按类别分组的 `index.md`

注意：

- 不要求 frontmatter
- 摘要优先取页面首个非标题段落
- 不自动推断复杂元数据

### 5.3 `llmwiki log append`

作用：

- 以统一标题格式追加条目

格式：

```md
## [2026-04-09] ingest | Source title
- files: wiki/sources/foo.md, wiki/entities/bar.md
- note: created summary and updated related pages
```

### 5.4 `llmwiki lint`

只做机械检查：

- 断裂的 `[[Wiki Links]]`
- 重复标题
- 空页面
- `index.md` 是否过期
- `log.md` 标题格式是否一致
- orphan page（无入链）

不做：

- 事实冲突判定
- 来源可信度判断
- stale claims 推理
- 缺失概念的语义发现

这些都留给外部 agent 的 `lint` 工作流。

### 5.5 `llmwiki prompt ingest <raw-path>`

输出一段可直接复制给 agent 的提示词，包含：

- 本次原始来源路径
- 要遵守的 `AGENTS.md`
- 需要先读 `index.md`
- 需要写 summary page、更新相关页面、更新 `index.md`、追加 `log.md`

### 5.6 `llmwiki prompt query "<question>"`

输出一段提示词，要求 agent：

- 先读 `index.md`
- 找出相关页面
- 基于已有 wiki 回答
- 如答案有长期价值，可另存为新页面
- 若写入新页面，需更新 `index.md` 与 `log.md`

### 5.7 `llmwiki prompt lint`

输出一段提示词，要求 agent：

- 通读 `index.md` 与近期 `log.md`
- 查找矛盾、陈旧结论、孤立页面、缺失交叉引用、信息缺口
- 形成 health-check 结果
- 必要时提出新的 source acquisition 建议

## 6. AGENTS.md 的定位

`AGENTS.md` 是纯内核版中唯一真正的“schema”。它不需要 JSON Schema，也不需要 frontmatter 契约。它只需要写清楚：

- `raw/` 永远只读
- `index.md` 每次 ingest/query-save 后都要更新
- `log.md` 只能追加，不得重写历史
- 页面应使用 Markdown 与 `[[Wiki Links]]`
- query 时先读 `index.md`
- lint 时重点看 contradictions / stale claims / orphan pages / missing pages

也就是说，**纪律来自 agent instruction，而不是机器强约束**。

## 7. 实现结构

纯内核版建议只用一个 binary crate：

```text
src/
├─ main.rs
├─ cli.rs
├─ repo.rs
├─ scan.rs
├─ markdown.rs
├─ index.rs
├─ logbook.rs
├─ lint.rs
└─ prompt.rs
```

### 模块职责

- `repo.rs`：仓库根目录发现、路径拼接、骨架初始化
- `scan.rs`：扫描 `wiki/` 页面
- `markdown.rs`：提取标题、首段摘要、`[[Wiki Links]]`
- `index.rs`：重建 `index.md`
- `logbook.rs`：追加 `log.md`
- `lint.rs`：机械健康检查
- `prompt.rs`：生成三类工作提示

## 8. 依赖选择

保持最小：

- `clap`：命令行解析
- `ignore`：扫描仓库并尊重 `.gitignore`
- `camino`：UTF-8 路径抽象，兼容 Windows
- `regex`：解析 `[[Wiki Links]]` 与日志标题
- `chrono`：生成日志日期
- `anyhow` 或 `miette`：错误输出
- `serde` 仅用于 CLI JSON 输出（若需要）

不引入：

- 数据库
- 异步运行时
- JSON Schema
- Git 库
- Markdown AST 重解析器

原因很简单：纯内核版的任务量不足以证明这些依赖的必要性。

## 9. Windows 约束

虽然是纯内核版，仍需从第一天支持 Windows：

- 内部一律使用 repo-relative UTF-8 路径
- 写入 `index.md` 时统一使用 `/`
- 实际落盘由 `camino` / `std::path` 适配
- 不依赖 shell-specific 命令
- 示例命令全部使用 CLI 子命令，而非 bash 管道

## 10. 页面约定

不强制 frontmatter。

最小页面格式：

```md
# Title

One paragraph summary.

## Notes
...

## Links
- [[entities/foo]]
- [[concepts/bar]]
```

说明：

- 标题必须存在
- 第一段默认作为 `index.md` 的 one-line summary 来源
- 页面间关系主要通过 `[[Wiki Links]]` 建立

## 11. Lint 范围

### 11.1 机械 lint

- `LINK001`：`[[link]]` 无目标页面
- `PAGE001`：页面缺失 H1 标题
- `PAGE002`：页面无正文
- `IDX001`：`index.md` 未覆盖现有页面
- `LOG001`：`log.md` 标题格式不一致
- `ORPHAN001`：页面没有任何入链

### 11.2 非机械 lint（只通过 prompt 交给 agent）

- `contradiction`：页面间结论冲突
- `stale claim`：旧说法被新来源推翻
- `gap`：重要概念尚无独立页面
- `missing cross-reference`：应建立但未建立的关系

## 12. Git 的位置

在纯内核版中，Git 只是仓库承载层，不是工作流引擎。

只保留三点：

- 仓库应当是一个 Git repo
- `index.md`、`log.md`、`wiki/` 的变更都由普通提交记录
- 审阅使用标准 `git diff`

不内置：

- hooks
- protected branch 规则
- 托管平台 API
- review packet 生成器

## 13. 开发顺序

### M0：骨架

完成：

- `llmwiki init`
- 最小目录与模板
- Windows/macOS/Linux 路径通过

### M1：索引

完成：

- 页面扫描
- 标题与首段摘要提取
- `index rebuild`

### M2：日志

完成：

- `log append`
- `recent`

### M3：机械 lint

完成：

- `[[link]]` 解析
- orphan / empty / missing title / stale index 检查

### M4：prompt bridge

完成：

- `prompt ingest`
- `prompt query`
- `prompt lint`

此时项目即达到纯 Karpathy 内核版可用状态。

## 14. 验收标准

满足以下条件即可判定为完成：

1. 新建仓库后，agent 能只依赖 `AGENTS.md + index.md + log.md + wiki/ + raw/` 开始工作。
2. CLI 可稳定重建 `index.md`。
3. CLI 可按统一格式追加 `log.md`。
4. CLI 可发现断链、空页、孤页与过期索引。
5. CLI 可生成可直接复制给外部 agent 的 ingest/query/lint 提示词。
6. 全流程不依赖任何特定模型 API 或托管平台。

## 15. 一句话原则

**Rust 只负责把仓库维护动作做稳；知识提炼、综合与写作仍由外部 agent 负责。**
