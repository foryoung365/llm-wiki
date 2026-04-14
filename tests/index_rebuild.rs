mod support;

#[test]
fn rebuild_index_groups_pages_by_type() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/entities/agent.md",
        r#"---
page_type: entity
title: Agent
slug: agent
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs: []
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Agent

可执行任务的代理。
"#,
    );
    support::write_file(
        &repo,
        "wiki/concepts/wiki.md",
        r#"---
page_type: concept
title: Wiki
slug: wiki
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs: []
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Wiki

用于组织知识的结构化空间。
"#,
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("rebuild-index");
    cmd.assert().success();

    let index = support::read_file(&repo, "wiki/_meta/index.md");
    assert!(index.contains("## Entities"));
    assert!(index.contains("[[entities/agent]]"));
    assert!(index.contains("## Concepts"));
    assert!(index.contains("[[concepts/wiki]]"));
}
