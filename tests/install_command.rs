use predicates::str::contains;

#[test]
fn install_copies_current_binary_to_shared_path_override() {
    let repo = tempfile::tempdir().expect("tempdir");
    let shared_cli = repo.path().join(if cfg!(windows) {
        "shared-llmwiki.exe"
    } else {
        "shared-llmwiki"
    });

    let mut cmd = assert_cmd::Command::cargo_bin("llmwiki").expect("binary exists");
    cmd.current_dir(repo.path())
        .arg("install")
        .env("LLMWIKI_INSTALL_PATH", &shared_cli);
    cmd.assert()
        .success()
        .stdout(contains("Installed shared CLI"))
        .stdout(contains(shared_cli.display().to_string()));

    assert!(shared_cli.exists());
}
