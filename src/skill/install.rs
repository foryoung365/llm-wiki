use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::install::{self, InstallAction, InstallSummary};
use crate::repo::Repo;

use super::render;
use super::targets;
use super::{ALL_HARNESSES, SkillHarness, SkillScope};

const AGENTS_SKILL_BLOCK_START: &str = "<!-- BEGIN LLMWIKI REPO SKILLS -->";
const AGENTS_SKILL_BLOCK_END: &str = "<!-- END LLMWIKI REPO SKILLS -->";

#[derive(Clone, Debug)]
pub struct SkillInstallRequest {
    pub harness: SkillHarness,
    pub scope: SkillScope,
    pub force: bool,
}

#[derive(Clone, Debug)]
pub struct SkillInstallSummary {
    pub harness: SkillHarness,
    pub scope: SkillScope,
    pub skill_dir: std::path::PathBuf,
    pub cli: InstallSummary,
}

pub fn install_skill(
    repo: Option<&Repo>,
    request: SkillInstallRequest,
) -> Result<SkillInstallSummary> {
    let cli = install::install_current_binary(request.force)?;
    let bundle = render::render_bundle(request.harness, request.scope)?;
    let skill_dir = targets::resolve_target_dir(repo, request.harness, request.scope)?;

    if skill_dir.exists() && request.force {
        fs::remove_dir_all(&skill_dir).with_context(|| {
            format!(
                "failed to replace existing skill directory: {}",
                skill_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("failed to create skill directory: {}", skill_dir.display()))?;

    let base_dir = skill_dir.parent().unwrap_or(skill_dir.as_path());
    for file in bundle.files {
        let destination = base_dir.join(&file.relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create skill parent directory: {}",
                    parent.display()
                )
            })?;
        }
        if !destination.exists() || request.force {
            fs::write(&destination, &file.contents).with_context(|| {
                format!("failed to write skill file: {}", destination.display())
            })?;
        }
        mark_executable_if_needed(&destination, file.executable)?;
    }

    if matches!(request.scope, SkillScope::Repo) {
        if let Some(repo) = repo {
            sync_repo_agents_skills(repo)?;
        }
    }

    Ok(SkillInstallSummary {
        harness: request.harness,
        scope: request.scope,
        skill_dir,
        cli,
    })
}

fn mark_executable_if_needed(path: &Path, executable: bool) -> Result<()> {
    if !executable {
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .with_context(|| format!("failed to read permissions: {}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("failed to mark script executable: {}", path.display()))?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

fn sync_repo_agents_skills(repo: &Repo) -> Result<()> {
    let agents_path = PathBuf::from(repo.agents_file().as_str());
    let contents = fs::read_to_string(&agents_path)
        .with_context(|| format!("failed to read AGENTS.md: {}", agents_path.display()))?;
    let block = render_agents_skill_block(repo)?;
    let updated = replace_or_append_managed_block(&contents, &block);

    if updated != contents {
        fs::write(&agents_path, updated)
            .with_context(|| format!("failed to update AGENTS.md: {}", agents_path.display()))?;
    }

    Ok(())
}

fn render_agents_skill_block(repo: &Repo) -> Result<String> {
    let mut installed = Vec::new();
    for harness in ALL_HARNESSES {
        let skill_dir = targets::resolve_target_dir(Some(repo), harness, SkillScope::Repo)?;
        if skill_dir.join("SKILL.md").exists() {
            installed.push((harness, path_relative_to_repo(repo, &skill_dir)?));
        }
    }

    if installed.is_empty() {
        return Ok(String::new());
    }

    let mut lines = vec![
        AGENTS_SKILL_BLOCK_START.to_string(),
        "## 15. 已安装 Skills".to_string(),
        String::new(),
        "本仓库已安装以下项目级 skill；若当前 harness 支持项目级技能发现，应优先使用对应入口。"
            .to_string(),
        String::new(),
    ];

    for (harness, path) in installed {
        lines.push(format!("- {}：`{}`", harness, path));
    }

    lines.push(String::new());
    lines.push("说明：".to_string());
    lines.push("- 这些 skill 通过共享 `llmwiki` CLI 执行确定性动作。".to_string());
    lines.push("- 仓库工作流仍以本文件与各 skill 的 `SKILL.md` 为准。".to_string());
    lines.push(AGENTS_SKILL_BLOCK_END.to_string());

    Ok(lines.join("\n"))
}

fn replace_or_append_managed_block(original: &str, block: &str) -> String {
    let trimmed_original = original.trim_end_matches('\n');
    if let (Some(start), Some(end_marker_pos)) = (
        original.find(AGENTS_SKILL_BLOCK_START),
        original.find(AGENTS_SKILL_BLOCK_END),
    ) {
        let end = end_marker_pos + AGENTS_SKILL_BLOCK_END.len();
        let mut updated = String::new();
        updated.push_str(original[..start].trim_end_matches('\n'));
        if !block.is_empty() {
            updated.push_str("\n\n");
            updated.push_str(block);
        }
        let suffix = original[end..].trim_matches('\n');
        if !suffix.is_empty() {
            updated.push_str("\n\n");
            updated.push_str(suffix);
        }
        updated.push('\n');
        return updated;
    }

    if block.is_empty() {
        let mut untouched = trimmed_original.to_string();
        untouched.push('\n');
        return untouched;
    }

    let mut updated = trimmed_original.to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(block);
    updated.push('\n');
    updated
}

fn path_relative_to_repo(repo: &Repo, path: &Path) -> Result<String> {
    let root = PathBuf::from(repo.root().as_str());
    let relative = path
        .strip_prefix(&root)
        .map_err(|_| anyhow!("path is outside repo root: {}", path.display()))?;
    Ok(relative.display().to_string().replace('\\', "/"))
}

pub fn describe_install_action(action: InstallAction) -> &'static str {
    match action {
        InstallAction::Installed => "installed",
        InstallAction::Reused => "reused",
        InstallAction::Updated => "updated",
    }
}
