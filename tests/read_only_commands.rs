mod support;

use llmwiki::logbook;
use llmwiki::repo::Repo;
use predicates::str::contains;

#[test]
fn recent_and_list_show_expected_entries() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "wiki/questions/20260409-sample.md",
        r#"---
page_type: question
title: Sample Question
slug: sample-question
status: active
created_at: 2026-04-09
updated_at: 2026-04-09
source_refs: []
entity_refs: []
concept_refs: []
confidence: medium
review_after: 2026-05-09
---

# Sample Question

## 问题

这是一个问题页。
"#,
    );
    let repo_model = Repo::discover(Some(repo.path().to_str().expect("utf8"))).expect("repo");
    logbook::append_entry(
        &repo_model,
        "query",
        "recent-check",
        &[String::from("notes: recent command test")],
    )
    .expect("append log");

    let mut recent = support::command_for(&repo);
    recent.arg("recent").arg("--limit").arg("1");
    recent.assert().success().stdout(contains("recent-check"));

    let mut list = support::command_for(&repo);
    list.arg("list").arg("--page-type").arg("question");
    list.assert()
        .success()
        .stdout(contains("wiki/questions/20260409-sample.md"));
}
