mod support;

use predicates::str::contains;

#[test]
fn sync_state_writes_manifest_and_page_graph() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/sources/SRC-20260409-001-sample.md",
        r#"---
page_type: source
title: Sample Source
slug: src-20260409-001-sample
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs:
  - SRC-20260409-001
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Sample Source

这是来源摘要。

## 相关页面
- [[entities/sample-entity]]
"#,
    );
    support::write_file(
        &repo,
        "wiki/entities/sample-entity.md",
        r#"---
page_type: entity
title: Sample Entity
slug: sample-entity
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs:
  - SRC-20260409-001
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Sample Entity

实体摘要。
"#,
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("sync-state");
    cmd.assert()
        .success()
        .stdout(contains("State synchronized"));

    let manifest = support::read_file(&repo, "state/source_manifest.jsonl");
    let graph = support::read_file(&repo, "state/page_graph.json");

    assert!(manifest.contains("SRC-20260409-001"));
    assert!(graph.contains("\"key\": \"sources/SRC-20260409-001-sample\""));
    assert!(graph.contains("\"to\": \"entities/sample-entity\""));
}
