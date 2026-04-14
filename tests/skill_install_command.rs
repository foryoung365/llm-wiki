mod support;

use std::fs;

#[test]
fn skill_install_writes_codex_repo_skill_and_shared_cli() {
    let repo = support::init_repo();
    let shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut cmd = support::command_for(&repo);
    cmd.arg("skill")
        .arg("install")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    cmd.assert().success();

    assert!(shared_cli.exists());
    assert!(
        repo.path()
            .join(".agents/skills/llm-wiki/SKILL.md")
            .exists()
    );
    assert!(
        repo.path()
            .join(".agents/skills/llm-wiki/scripts/llmwikiw.sh")
            .exists()
    );
    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read agents");
    assert!(agents.contains("<!-- BEGIN LLMWIKI REPO SKILLS -->"));
    assert!(agents.contains(".agents/skills/llm-wiki"));
}

#[test]
fn skill_install_writes_user_scope_claude_skill() {
    let repo = support::init_repo();
    let home = tempfile::tempdir().expect("tempdir");
    let shared_cli = home.path().join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut cmd = support::command_for(&repo);
    cmd.arg("skill")
        .arg("install")
        .arg("--harness")
        .arg("claude")
        .arg("--scope")
        .arg("user")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli)
        .env("HOME", home.path())
        .env("USERPROFILE", home.path());
    cmd.assert().success();

    assert!(shared_cli.exists());
    assert!(
        home.path()
            .join(".claude/skills/llm-wiki/SKILL.md")
            .exists()
    );
    assert!(
        home.path()
            .join(".claude/skills/llm-wiki/scripts/llmwikiw.cmd")
            .exists()
    );
    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read agents");
    assert!(!agents.contains("<!-- BEGIN LLMWIKI REPO SKILLS -->"));
}

#[test]
fn skill_install_writes_openclaw_repo_skill_into_workspace_skills_dir() {
    let repo = support::init_repo();
    let shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut cmd = support::command_for(&repo);
    cmd.arg("skill")
        .arg("install")
        .arg("--harness")
        .arg("openclaw")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    cmd.assert().success();

    assert!(repo.path().join("skills/llm-wiki/SKILL.md").exists());
    assert!(
        repo.path()
            .join("skills/llm-wiki/scripts/llmwikiw.sh")
            .exists()
    );
    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read agents");
    assert!(agents.contains("skills/llm-wiki"));
}

#[test]
fn skill_install_preserves_existing_files_without_force() {
    let repo = support::init_repo();
    let shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut first = support::command_for(&repo);
    first
        .arg("skill")
        .arg("install")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    first.assert().success();

    let skill_md = repo.path().join(".agents/skills/llm-wiki/SKILL.md");
    let script = repo
        .path()
        .join(".agents/skills/llm-wiki/scripts/llmwikiw.sh");
    fs::write(&skill_md, "custom\n").expect("customize skill");
    fs::remove_file(&script).expect("remove script");

    let mut second = support::command_for(&repo);
    second
        .arg("skill")
        .arg("install")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    second.assert().success();

    assert_eq!(
        fs::read_to_string(&skill_md).expect("read skill"),
        "custom\n"
    );
    assert!(script.exists());
    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read agents");
    assert_eq!(
        agents.matches("<!-- BEGIN LLMWIKI REPO SKILLS -->").count(),
        1
    );
}
