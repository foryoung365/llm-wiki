mod support;

use predicates::str::contains;

#[test]
fn prepare_ingest_prints_agent_brief_with_suggested_source_id() {
    let repo = support::init_repo();
    support::write_file(&repo, "raw/inbox/agent-notes.md", "# Agent Notes\n");
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

关于代理的现有知识。
"#,
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("prepare-ingest").arg("raw/inbox/agent-notes.md");
    cmd.assert()
        .success()
        .stdout(contains("# llmwiki prepare-ingest brief"))
        .stdout(contains("建议来源 ID: SRC-"))
        .stdout(contains("wiki/sources/SRC-"));
}

#[test]
fn ask_command_is_not_available() {
    let repo = support::init_repo();
    let mut cmd = support::command_for(&repo);
    cmd.arg("ask");
    cmd.assert()
        .failure()
        .stderr(contains("unrecognized subcommand"));
}
