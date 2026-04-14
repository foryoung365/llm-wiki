use llmwiki::logbook;
use llmwiki::repo::Repo;

mod support;

#[test]
fn append_entry_adds_structured_log_heading() {
    let repo = support::init_repo();
    let repo_model = Repo::discover(Some(repo.path().to_str().expect("utf8"))).expect("repo");

    logbook::append_entry(
        &repo_model,
        "query",
        "test-question",
        &[String::from("pages_read: [wiki/entities/agent.md]")],
    )
    .expect("append log");

    let log = support::read_file(&repo, "wiki/_meta/log.md");
    assert!(log.contains("query | test-question"));
    assert!(log.contains("pages_read: [wiki/entities/agent.md]"));
}
