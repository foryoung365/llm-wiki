mod support;

use std::fs;

use predicates::str::contains;

#[test]
fn init_creates_starter_layout() {
    let repo = tempfile::tempdir().expect("tempdir");
    let mut cmd = assert_cmd::Command::cargo_bin("llmwiki").expect("binary exists");
    cmd.arg("init").arg(repo.path());
    cmd.assert()
        .success()
        .stdout(contains("Initialized repository"));

    assert!(repo.path().join("AGENTS.md").exists());
    assert!(repo.path().join("README.md").exists());
    assert!(repo.path().join("README.zh-CN.md").exists());
    assert!(repo.path().join("README.en.md").exists());
    assert!(repo.path().join("raw/inbox/.gitkeep").exists());
    assert!(repo.path().join("templates/README.md").exists());
    assert!(repo.path().join("wiki/_meta/index.md").exists());
    assert!(repo.path().join("wiki/_meta/log.md").exists());
}

#[test]
fn init_starts_with_empty_log() {
    let repo = tempfile::tempdir().expect("tempdir");
    let mut cmd = assert_cmd::Command::cargo_bin("llmwiki").expect("binary exists");
    cmd.arg("init").arg(repo.path());
    cmd.assert().success();

    let log = fs::read_to_string(repo.path().join("wiki/_meta/log.md")).expect("read log");
    assert_eq!(log, "# Log\n");

    let mut recent = assert_cmd::Command::cargo_bin("llmwiki").expect("binary exists");
    recent.arg("--repo").arg(repo.path()).arg("recent");
    recent
        .assert()
        .success()
        .stdout(contains("No log entries found."));
}

#[test]
fn init_can_install_codex_skill_on_demand() {
    let repo = tempfile::tempdir().expect("tempdir");
    let shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut cmd = assert_cmd::Command::cargo_bin("llmwiki").expect("binary exists");
    cmd.arg("init")
        .arg(repo.path())
        .arg("--install-skill")
        .arg("codex")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    cmd.assert().success().stdout(contains("Installed skills:"));

    assert!(shared_cli.exists());
    assert!(
        repo.path()
            .join(".agents/skills/llm-wiki/SKILL.md")
            .exists()
    );
    assert!(
        repo.path()
            .join(".agents/skills/llm-wiki/scripts/llmwikiw.cmd")
            .exists()
    );
    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read agents");
    assert!(agents.contains("<!-- BEGIN LLMWIKI REPO SKILLS -->"));
    assert!(agents.contains(".agents/skills/llm-wiki"));
}
