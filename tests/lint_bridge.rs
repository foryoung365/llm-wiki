mod support;

use predicates::str::contains;

#[test]
fn lint_appends_log_and_prints_semantic_follow_up() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/concepts/test.md",
        r#"---
page_type: concept
title: Test Concept
slug: test-concept
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs: []
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Test Concept

用于 lint 桥接测试。
"#,
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("lint");
    cmd.assert()
        .success()
        .stdout(contains("请让 agent 继续执行语义 lint"));

    let log = support::read_file(&repo, "wiki/_meta/log.md");
    assert!(log.contains("lint | mechanical-health-check"));
}

#[test]
fn lint_logs_actual_contradictions_and_gaps() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/concepts/disputed.md",
        r#"---
page_type: concept
title: Disputed Concept
slug: disputed-concept
status: disputed
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs:
  - SRC-20260409-001
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Disputed Concept

## 当前认识

来源存在冲突。

## 相关页面

- [[concepts/missing]]
"#,
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("lint");
    cmd.assert().success();

    let log = support::read_file(&repo, "wiki/_meta/log.md");
    assert!(log.contains("contradictions: 1"));
    assert!(log.contains("gaps: 1"));
}
