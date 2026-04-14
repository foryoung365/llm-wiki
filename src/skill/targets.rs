use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::repo::Repo;

use super::{SkillHarness, SkillScope};

pub fn resolve_target_dir(
    repo: Option<&Repo>,
    harness: SkillHarness,
    scope: SkillScope,
) -> Result<PathBuf> {
    let base = match scope {
        SkillScope::Repo => repo_scope_base(repo, harness)?,
        SkillScope::User => user_scope_base(harness)?,
    };
    Ok(base.join("llm-wiki"))
}

fn repo_scope_base(repo: Option<&Repo>, harness: SkillHarness) -> Result<PathBuf> {
    let repo = repo.ok_or_else(|| {
        anyhow::anyhow!(
            "repo scope requires a discovered llm-wiki repository; run inside a repository or pass --repo"
        )
    })?;
    let root = PathBuf::from(repo.root().as_str());
    Ok(match harness {
        SkillHarness::Claude => root.join(".claude").join("skills"),
        SkillHarness::Opencode => root.join(".opencode").join("skills"),
        SkillHarness::Openclaw => root.join("skills"),
        SkillHarness::Codex => root.join(".agents").join("skills"),
    })
}

fn user_scope_base(harness: SkillHarness) -> Result<PathBuf> {
    let home = home_dir()?;
    Ok(match harness {
        SkillHarness::Claude => home.join(".claude").join("skills"),
        SkillHarness::Opencode => home.join(".config").join("opencode").join("skills"),
        SkillHarness::Openclaw => home.join(".openclaw").join("skills"),
        SkillHarness::Codex => home.join(".agents").join("skills"),
    })
}

fn home_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("HOME") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(path));
    }
    bail!("failed to resolve home directory for skill installation")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_scope_codex_targets_agents_skills() {
        let path =
            resolve_target_dir(None, SkillHarness::Codex, SkillScope::User).expect("target dir");
        assert!(path.ends_with(PathBuf::from(".agents").join("skills").join("llm-wiki")));
    }

    #[test]
    fn repo_scope_openclaw_targets_workspace_skills_dir() {
        let repo = crate::repo::Repo::for_init(Some("I:/tmp")).expect("repo");
        let path = resolve_target_dir(Some(&repo), SkillHarness::Openclaw, SkillScope::Repo)
            .expect("target dir");
        assert!(path.ends_with(PathBuf::from("skills").join("llm-wiki")));
    }
}
