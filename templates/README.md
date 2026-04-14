# 模板说明

`templates/` 用于约束 agent 在新建页面时采用统一结构，而不是自由发挥。

## 使用原则

- 新建页面时，优先从对应模板复制，并补齐 frontmatter。
- 模板字段允许增补，但不得删去 `AGENTS.md` 规定的必填字段。
- 若页面已有历史内容，应在保留既有证据与链接的前提下增量修改，而不是整页重写。
- `source_refs`、`entity_refs`、`concept_refs` 应与正文内容保持一致，不能只更新正文不更新 frontmatter。

## 模板映射

- `source-summary.md`：用于 `wiki/sources/` 的来源总结页
- `entity.md`：用于 `wiki/entities/` 的实体页
- `concept.md`：用于 `wiki/concepts/` 的概念页
- `question.md`：用于 `wiki/questions/` 的问题页
- `synthesis.md`：用于 `wiki/syntheses/` 的综合分析页
- `timeline.md`：用于 `wiki/timelines/` 的时间线页

## 维护要求

- 模板与 `AGENTS.md` 发生冲突时，以 `AGENTS.md` 为准，并同步修订模板。
- 模板的日期字段仅为示例值；实际写入时必须替换为当前日期。
- 模板中的 slug、来源 ID、路径占位符都必须替换，不得原样提交。

