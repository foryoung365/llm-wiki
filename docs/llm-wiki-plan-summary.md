# llm-wiki 可执行方案摘要

## 方案定位

将 Karpathy 的《LLM Wiki》实例化为一个“本地优先、Markdown 优先、Git 管理、LLM 维护”的知识系统。核心不是查询时临时拼接 RAG，而是让 LLM 持续维护一层介于原始资料与问答之间的持久化 Wiki。

## 我为你做出的具体化选择

### 1. 运行方式
- 先采用“无代码 MVP”：
  - Obsidian 负责浏览 Wiki
  - 代码代理负责写入与维护
  - 本仓库模板负责约束行为
- 再进入“CLI 自动化”：
- `llmwiki prepare-ingest`
- agent query workflow（按 `AGENTS.md` 执行）
  - `llmwiki lint`
  - `llmwiki rebuild-index`
  - `llmwiki sync-state`

### 2. 三层结构
- `raw/`：原始资料，绝不修改
- `wiki/`：来源页、实体页、概念页、问题页、综合页、时间线
- `AGENTS.md`：约束代理的 schema 与流程

### 3. 关键文件
- `wiki/_meta/index.md`：内容目录
- `wiki/_meta/log.md`：时间序日志
- `templates/*.md`：页面模板
- `state/`：缓存与派生索引，可删可重建

## MVP 目标

30 天内交付以下能力：

1. 单条来源 ingest
2. Wiki 问答
3. 问答结果落盘
4. 周期性 lint

## 执行节奏

### 第 1 周
- 建立仓库骨架
- 跑通 3 条来源的 supervised ingest
- 形成统一 frontmatter、命名与日志格式

### 第 2 周
- 固化 ingest / query / lint 工作流
- 明确哪些回答必须保存进 Wiki
- 建立 `source_manifest.jsonl`

### 第 3 周
- 设计 CLI 命令契约
- 自动解析 frontmatter、链接、来源引用
- 自动重建 `index.md` 与 `log.md`

### 第 4 周
- 在 Wiki 规模扩大后接入本地搜索
- 用 10–20 条真实来源试运行
- 建立 Git 审阅与回滚机制

## 最重要的 5 条规则

1. 不修改 `raw/`
2. 每次 ingest / query / lint 都更新 `index.md` 与 `log.md`
3. 所有页面都必须带可追溯 frontmatter
4. 高价值问答必须落盘，而不是留在聊天记录里
5. 自动化状态只能写入 `state/`，且必须可重建

## 交付物

你现在可以直接使用：
- `docs/llm-wiki-starter.zip`：完整 starter 包
- `AGENTS.md`：可直接交给代理执行的规则
- `docs/EXECUTION_PLAN_zh.md`：30 天执行计划
- `BACKLOG.csv`：优先级任务表
