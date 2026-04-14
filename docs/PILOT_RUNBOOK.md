# 试运行手册

本手册用于在 llm-wiki MVP 阶段组织一次真实试运行，验证仓库骨架、agent 工作流与 Rust CLI 是否已经形成可持续闭环。

## 试运行目标

- 用 10–20 条真实来源验证 `prepare-ingest` 流程
- 用 10 个真实问题验证 agent query 与问答落盘流程
- 用至少 2 轮 `lint` 验证机械检查与语义跟进机制
- 用 Git 记录每一轮知识变更，确保可追溯与可回滚

## 准备条件

- `llmwiki` 已可正常执行
- `AGENTS.md`、`templates/`、`wiki/_meta/` 已存在，并与当前 CLI 口径一致
- 试运行专题已经选定，来源范围可控，适合在 1–2 周内连续处理

## 推荐专题

- 一个长期研究主题，例如某类模型、某一技术方向、某一竞品集合
- 一个连续阅读对象，例如一本书、一门课程、一组论文
- 一个内部项目专题，例如需求、会议纪要、决策记录、复盘材料

## 单条来源处理流程

1. 将来源放入 `raw/inbox/`
2. 运行 `llmwiki sync-state`
3. 运行 `llmwiki prepare-ingest raw/inbox/<file>`
4. 将输出任务简报交给 agent 执行
5. 审阅以下变更：
   - `wiki/sources/`
   - `wiki/entities/`
   - `wiki/concepts/`
   - `wiki/syntheses/`
   - `wiki/timelines/`
   - `wiki/_meta/index.md`
   - `wiki/_meta/log.md`
6. 若内容合格，再提交到 Git

## 提问与落盘流程

1. 由 agent 按 `AGENTS.md` 的 Query 工作流处理问题
2. 审阅 agent 的答案、引用页面与不确定性说明
3. 判断答案是否具有长期价值
4. 若有长期价值，要求 agent 保存到：
   - `wiki/questions/YYYYMMDD-<slug>.md`
   - 或 `wiki/syntheses/<slug>.md`
5. 确认 `wiki/_meta/index.md` 与 `wiki/_meta/log.md` 已同步更新

## 每周健康检查

1. 运行 `llmwiki lint`
2. 先处理机械问题：
   - 断链
   - 空页
   - 缺失 H1
   - 过期 `review_after`
   - 孤页
   - index 漏页
3. 再把语义跟进简报交给 agent，检查：
   - contradictions
   - stale claims
   - missing cross-references
   - gaps

## 试运行通过标准

- 至少 10 条来源已完成来源页与知识页更新
- 至少 10 个问题中，有 5 个以上被沉淀为问题页或综合页
- `state/` 删除后，可通过 `sync-state` 与 `rebuild-index` 完整恢复
- `lint` 可以稳定输出 JSON 报告，并追加日志
- 主要回答开始依赖 wiki 页面，而不是临时重做一次原始资料检索

## 试运行结束后的决策

- 若 index-first + `state/` 已足够，应继续扩大内容规模，而不是提前引入搜索系统
- 若问答召回开始明显不足，再进入 `docs/SEARCH_SCALING.md` 中的扩容评估
- 若模板或 `AGENTS.md` 经常被修改，应先稳住规范，再扩大吞吐
