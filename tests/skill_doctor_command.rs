mod support;

use std::fs;
use std::path::PathBuf;

use assert_cmd::cargo::cargo_bin;
use predicates::str::contains;

#[test]
fn skill_doctor_reports_installed_codex_repo_skill() {
    let repo = support::init_repo();
    let shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut install = support::command_for(&repo);
    install
        .arg("skill")
        .arg("install")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    install.assert().success();

    let mut doctor = support::command_for(&repo);
    doctor
        .arg("skill")
        .arg("doctor")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    doctor
        .assert()
        .success()
        .stdout(contains("shared-cli [ok]"))
        .stdout(contains("codex-repo [ok]"));
}

#[test]
fn skill_doctor_reports_missing_skill_files() {
    let repo = support::init_repo();
    let missing_shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });
    support::write_file(
        &repo,
        ".agents/skills/llm-wiki/SKILL.md",
        "---\nname: llm-wiki\n---\n",
    );

    let mut doctor = support::command_for(&repo);
    doctor
        .arg("skill")
        .arg("doctor")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("PATH", "")
        .env("LLMWIKI_INSTALL_PATH", &missing_shared_cli);
    doctor
        .assert()
        .success()
        .stdout(contains("shared-cli [warn]"))
        .stdout(contains("codex-repo [warn]"))
        .stdout(contains("scripts/llmwikiw.cmd"))
        .stdout(contains("scripts/llmwikiw.sh"));
}

#[test]
fn skill_doctor_reports_llmwiki_bin_as_ok() {
    let repo = support::init_repo();
    let missing_shared_cli = repo.path().join(".tmp").join(if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    });

    let mut doctor = support::command_for(&repo);
    doctor
        .arg("skill")
        .arg("doctor")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env("LLMWIKI_BIN", cargo_bin("llmwiki"))
        .env("LLMWIKI_INSTALL_PATH", &missing_shared_cli);
    doctor
        .assert()
        .success()
        .stdout(contains("shared-cli [ok]"))
        .stdout(contains("resolved via LLMWIKI_BIN"));
}

#[test]
fn skill_doctor_reports_path_binary_as_ok() {
    let repo = support::init_repo();
    let path_dir = tempfile::tempdir().expect("tempdir");
    let command_name = if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    };
    let path_binary = path_dir.path().join(command_name);
    fs::copy(cargo_bin("llmwiki"), &path_binary).expect("copy llmwiki binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&path_binary).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path_binary, permissions).expect("set executable");
    }
    let current_path = std::env::var_os("PATH").expect("PATH");
    let combined_path =
        std::env::join_paths([PathBuf::from(path_dir.path()), PathBuf::from(current_path)])
            .expect("join path");
    let missing_shared_cli = repo.path().join(".tmp").join(command_name);

    let mut doctor = support::command_for(&repo);
    doctor
        .arg("skill")
        .arg("doctor")
        .arg("--harness")
        .arg("codex")
        .arg("--scope")
        .arg("repo")
        .env_remove("LLMWIKI_BIN")
        .env("PATH", &combined_path)
        .env("LLMWIKI_INSTALL_PATH", &missing_shared_cli);
    doctor
        .assert()
        .success()
        .stdout(contains("shared-cli [ok]"))
        .stdout(contains("resolved via PATH"));
}
