use std::env;
use std::path::{Path, PathBuf};

use crate::install;
use crate::repo::Repo;

use super::targets;
use super::{ALL_HARNESSES, SkillHarness, SkillScope};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoctorStatus {
    Ok,
    Warn,
}

#[derive(Clone, Debug)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

pub fn run(repo: Option<&Repo>, harness: Option<SkillHarness>, scope: SkillScope) -> DoctorReport {
    let mut checks = Vec::new();
    checks.push(shared_cli_check());

    let harnesses = match harness {
        Some(harness) => vec![harness],
        None => ALL_HARNESSES.to_vec(),
    };

    for harness in harnesses {
        checks.push(skill_check(repo, harness, scope));
    }

    DoctorReport { checks }
}

fn shared_cli_check() -> DoctorCheck {
    if let Some(path) = env::var_os("LLMWIKI_BIN").map(PathBuf::from) {
        return if path.exists() {
            DoctorCheck {
                name: "shared-cli".to_string(),
                status: DoctorStatus::Ok,
                detail: format!("resolved via LLMWIKI_BIN at {}", path.display()),
            }
        } else {
            DoctorCheck {
                name: "shared-cli".to_string(),
                status: DoctorStatus::Warn,
                detail: format!("LLMWIKI_BIN points to a missing path: {}", path.display()),
            }
        };
    }

    let shared_path = install::shared_cli_path().ok();
    if let Some(path) = shared_path.as_ref().filter(|path| path.exists()) {
        return DoctorCheck {
            name: "shared-cli".to_string(),
            status: DoctorStatus::Ok,
            detail: format!("found shared install at {}", path.display()),
        };
    }

    if let Some(path) = find_on_path("llmwiki") {
        return DoctorCheck {
            name: "shared-cli".to_string(),
            status: DoctorStatus::Ok,
            detail: format!("resolved via PATH at {}", path.display()),
        };
    }

    let detail = match shared_path {
        Some(path) => format!("missing at {}; run `llmwiki install` first", path.display()),
        None => "shared install path could not be resolved and llmwiki was not found on PATH"
            .to_string(),
    };

    DoctorCheck {
        name: "shared-cli".to_string(),
        status: DoctorStatus::Warn,
        detail,
    }
}

fn skill_check(repo: Option<&Repo>, harness: SkillHarness, scope: SkillScope) -> DoctorCheck {
    let name = format!("{harness}-{scope}");
    let skill_dir = match targets::resolve_target_dir(repo, harness, scope) {
        Ok(path) => path,
        Err(error) => {
            return DoctorCheck {
                name,
                status: DoctorStatus::Warn,
                detail: error.to_string(),
            };
        }
    };

    let mut missing = Vec::new();
    for required in required_paths(harness, &skill_dir) {
        if !required.exists() {
            missing.push(relative_from(&skill_dir, &required));
        }
    }

    if missing.is_empty() {
        DoctorCheck {
            name,
            status: DoctorStatus::Ok,
            detail: format!("found at {}", skill_dir.display()),
        }
    } else {
        DoctorCheck {
            name,
            status: DoctorStatus::Warn,
            detail: format!(
                "missing {} under {}",
                missing.join(", "),
                skill_dir.display()
            ),
        }
    }
}

fn required_paths(harness: SkillHarness, skill_dir: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        skill_dir.join("SKILL.md"),
        skill_dir.join("scripts").join("llmwikiw.cmd"),
        skill_dir.join("scripts").join("llmwikiw.sh"),
    ];
    if matches!(harness, SkillHarness::Claude) {
        paths.push(skill_dir.join("templates").join(".gitkeep"));
    }
    paths
}

fn relative_from(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let pathexts = windows_path_exts();
    for directory in env::split_paths(&path) {
        for candidate in command_candidates(&directory, command, &pathexts) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn command_candidates(directory: &Path, command: &str, pathexts: &[String]) -> Vec<PathBuf> {
    let command_path = Path::new(command);
    if command_path.extension().is_some() || !cfg!(windows) {
        return vec![directory.join(command)];
    }

    let mut candidates = Vec::new();
    candidates.push(directory.join(command));
    for extension in pathexts {
        candidates.push(directory.join(format!("{command}{extension}")));
    }
    candidates
}

fn windows_path_exts() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }

    env::var("PATHEXT")
        .ok()
        .map(|value| {
            value
                .split(';')
                .filter(|item| !item.is_empty())
                .map(|item| item.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| {
            vec![
                ".com".to_string(),
                ".exe".to_string(),
                ".bat".to_string(),
                ".cmd".to_string(),
            ]
        })
}
