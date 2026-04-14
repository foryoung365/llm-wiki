#![allow(dead_code)]

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

pub fn init_repo() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("llmwiki").expect("binary exists");
    cmd.arg("init").arg(dir.path());
    cmd.assert().success();
    dir
}

pub fn command_for(repo: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("llmwiki").expect("binary exists");
    cmd.arg("--repo").arg(repo.path());
    cmd
}

pub fn write_file(repo: &TempDir, relative: &str, contents: &str) {
    write_path(&repo.path().join(relative), contents);
}

pub fn write_path(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, contents).expect("write file");
}

pub fn read_file(repo: &TempDir, relative: &str) -> String {
    fs::read_to_string(repo.path().join(relative)).expect("read file")
}
