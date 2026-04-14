mod support;

use llmwiki::lint;
use llmwiki::repo::Repo;

#[test]
fn lint_reports_broken_links_and_writes_json_report() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/entities/broken.md",
        r#"---
page_type: entity
title: Broken Entity
slug: broken-entity
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs: []
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-04-01
---

# Broken Entity

一个带有断链的实体页。

## 相关页面
- [[concepts/missing]]
"#,
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("lint").arg("--no-log");
    cmd.assert().success();

    let lint_json = support::read_file(&repo, "state/lint-latest.json");
    assert!(lint_json.contains("\"LINK001\""));
    assert!(lint_json.contains("\"STALE001\""));
    assert!(lint_json.contains("\"ORPHAN001\""));
}

#[test]
fn lint_does_not_duplicate_parser_diagnostics() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/entities/no-heading.md",
        r#"---
page_type: entity
title: No Heading
slug: no-heading
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs: []
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

正文存在，但没有 H1。
"#,
    );

    let repo_model = Repo::discover(Some(repo.path().to_str().expect("utf8"))).expect("repo");
    let report = lint::run(&repo_model, false).expect("lint report");

    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PAGE001")
            .count(),
        1
    );
    assert_eq!(report.summary.parser_diagnostics, 1);
}
