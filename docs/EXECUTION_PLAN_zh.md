# 30 天执行计划（可直接照单推进）

## 总目标

在 30 天内交付一个可用的 llm-wiki MVP，满足以下四个场景：

1. 单条来源 ingest
2. 基于 wiki 的问答
3. 问答结果回写 wiki
4. 周期性 lint 健康检查

## 执行原则

- 先闭环，后自动化
- 先小规模高质量 ingest，再扩大吞吐
- 先 index，再 search
- 先人审，再批处理

## 第 1 周：建立可运行骨架

### Day 1
- 初始化 Git 仓库
- 建立目录结构
- 放入 `AGENTS.md`
- 创建 `wiki/_meta/index.md` 与 `wiki/_meta/log.md`
- 建立 `templates/` 模板
- 建立 `templates/README.md`

完成标准：
- 代理可读取规则
- 目录与模板齐备
- 可以开始人工 ingest

### Day 2
- 选 3 份高质量来源放入 `raw/inbox/`
- 执行 1 次 supervised ingest
- 审阅页面结构、命名与来源追溯效果
- 修订 `AGENTS.md`

完成标准：
- 至少生成 1 个来源总结页
- 至少更新 2 个实体/概念页
- `index.md` 与 `log.md` 正常更新

### Day 3
- 再 ingest 2–3 条来源
- 校准 frontmatter 字段
- 定义 `status` 与 `confidence` 语义
- 确定 `review_after` 策略

完成标准：
- 形成统一页面样式
- 不再频繁改模板字段

### Day 4
- 设计问题页和综合页模板
- 发起 3 个真实问题
- 将高价值回答保存为 `wiki/questions/`

完成标准：
- 至少有 2 个保存后的问题页
- 回答可追溯到现有 wiki 页面

### Day 5
- 手工执行首轮 lint
- 整理 contradictions / stale / orphan / gaps
- 根据 lint 结果修补 wiki

完成标准：
- `log.md` 中出现首条 lint 记录
- 至少修复 3 个链接或结构问题

## 第 2 周：固化工作流

### Day 6–7
- 将 ingest 流程沉淀为标准操作说明
- 明确输入、输出、必更文件与审阅点
- 建立 `source_manifest.jsonl`

完成标准：
- 任一新来源都能按同一路径处理
- 来源 ID 生成规则不再变化

### Day 8–9
- 将 query 流程标准化
- 规定何种回答必须落盘
- 形成“回答后写回 wiki”的习惯

完成标准：
- 高价值问题不再只停留在对话窗口
- 问题页命名规则稳定

### Day 10
- 将 lint 流程标准化
- 明确四类问题的处理策略
- 形成每周例行检查节奏

完成标准：
- 可重复执行 weekly lint
- 每类 lint 结果都有对应动作

## 第 3 周：引入工程自动化

### Day 11–12
- 以 provider-neutral Rust CLI 设计命令接口：
  - `llmwiki init`
- `llmwiki prepare-ingest`
- agent query workflow（按 `AGENTS.md` 执行）
  - `llmwiki lint`
  - `llmwiki rebuild-index`
  - `llmwiki sync-state`
  - `llmwiki recent`
  - `llmwiki list`

完成标准：
- 命令参数与输出约定确定
- 代理可据此实现脚本

### Day 13–14
- 实现 frontmatter 解析、链接扫描、来源引用扫描
- 将结果写入 `state/source_manifest.jsonl`、`state/page_graph.json` 或同等派生状态

完成标准：
- 页面图谱与来源图谱可被机器读取
- 不依赖人工检查即可输出基础统计

### Day 15
- 自动重建 `index.md`
- 自动追加 `log.md`
- 自动生成 lint JSON 报告
- 验证 `state/` 删除后可完整重建

完成标准：
- 元数据维护开始自动化
- 日志与索引不再靠手工更新

## 第 4 周：检索增强与试运行

### Day 16–18
- 当 wiki 页数较少时，仅保留 index-first 模式
- 当页数扩大后，接入本地搜索引擎
- 验证关键词检索、语义检索、混合检索效果

完成标准：
- 代理不再盲扫整个仓库
- 查询相关页的召回质量明显提升

### Day 19–21
- 设计试运行专题（例如一个项目、一个研究话题、一个竞品集合）
- 连续 ingest 10–20 个真实来源
- 连续回答 10 个真实问题

完成标准：
- Wiki 不只是样板，而是承载真实工作
- 能看到实体页与概念页逐步成熟

### Day 22–24
- 执行 2 轮 lint
- 修复冲突、孤页、缺页
- 补足 3 个综合页或时间线页

完成标准：
- Wiki 图谱开始形成“中心页—主题页—来源页”结构
- 问答质量依赖 wiki，而不是依赖临时记忆

### Day 25–27
- 引入 Git 审阅习惯
- 为 ingest / query / lint 分别使用标准 commit message
- 试行分支审阅或 PR 审阅

完成标准：
- 每次知识变更可回溯
- 错误修改可回滚

### Day 28–30
- 汇总 MVP 指标
- 确认是否进入下一阶段
- 形成 V1.0 路线图

完成标准：
- 有 20+ 来源
- 有 50+ wiki 页
- 有稳定 query / ingest / lint 节奏
- 有一份清晰的迭代清单

## MVP 验收标准

MVP 通过需同时满足：

1. 任意新来源可在一次 ingest 中完成：
   - 来源总结
   - 至少 1 个知识页更新
   - index 更新
   - log 追加

2. 任意真实问题可：
   - 基于 wiki 回答
   - 指出依据页面
   - 保存为问题页或综合页

3. 每周至少一次 lint，可输出：
   - contradictions
   - stale
   - orphans
   - gaps

4. 所有知识变更都在 Git 中可追溯
5. `state/` 删除后，可由 `sync-state` 与 `rebuild-index` 恢复

## 资源配置建议

最小配置：
- 1 名负责人
- 1 个代码代理
- 1 个 Markdown 仓库
- 1 个固定每周维护时段

推荐配置：
- 单人研究：每天 30–60 分钟
- 小团队：每周 2 次 review
