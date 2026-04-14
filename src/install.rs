use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallAction {
    Installed,
    Reused,
    Updated,
}

#[derive(Clone, Debug)]
pub struct InstallSummary {
    pub path: PathBuf,
    pub source_path: PathBuf,
    pub action: InstallAction,
}

pub fn install_current_binary(force: bool) -> Result<InstallSummary> {
    let source_path = env::current_exe().context("failed to resolve current llmwiki binary")?;
    let target_path = shared_cli_path()?;

    if same_path(&source_path, &target_path) {
        return Ok(InstallSummary {
            path: target_path,
            source_path,
            action: InstallAction::Reused,
        });
    }

    if target_path.exists() && !force {
        return Ok(InstallSummary {
            path: target_path,
            source_path,
            action: InstallAction::Reused,
        });
    }

    let parent = target_path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "failed to determine install directory for shared llmwiki binary: {}",
            target_path.display()
        )
    })?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "failed to create shared install directory: {}",
            parent.display()
        )
    })?;

    let temp_path = temp_install_path(&target_path);
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    fs::copy(&source_path, &temp_path).with_context(|| {
        format!(
            "failed to copy current llmwiki binary from {} to {}",
            source_path.display(),
            temp_path.display()
        )
    })?;
    mark_executable(&temp_path)?;

    if target_path.exists() {
        fs::remove_file(&target_path).with_context(|| {
            format!(
                "failed to replace existing shared llmwiki binary: {}",
                target_path.display()
            )
        })?;
    }
    fs::rename(&temp_path, &target_path).with_context(|| {
        format!(
            "failed to move llmwiki binary into shared location: {}",
            target_path.display()
        )
    })?;
    mark_executable(&target_path)?;

    Ok(InstallSummary {
        path: target_path,
        source_path,
        action: if force {
            InstallAction::Updated
        } else {
            InstallAction::Installed
        },
    })
}

pub fn shared_cli_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("LLMWIKI_INSTALL_PATH") {
        return Ok(PathBuf::from(path));
    }

    let base_dir = match env::consts::OS {
        "windows" => local_app_data_dir()?,
        _ => xdg_data_dir()?,
    };
    let filename = if cfg!(windows) {
        "llmwiki.exe"
    } else {
        "llmwiki"
    };
    Ok(base_dir.join("llmwiki").join("bin").join(filename))
}

fn local_app_data_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path));
    }
    let home = home_dir()?;
    Ok(home.join("AppData").join("Local"))
}

fn xdg_data_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".local").join("share"))
}

fn home_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("HOME") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(path));
    }
    bail!("failed to resolve home directory")
}

fn temp_install_path(target_path: &Path) -> PathBuf {
    let filename = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("llmwiki");
    target_path.with_file_name(format!("{filename}.tmp"))
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn mark_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .with_context(|| format!("failed to read permissions: {}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("failed to mark binary executable: {}", path.display()))?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_cli_path_uses_platform_specific_root() {
        let path = shared_cli_path().expect("shared cli path");

        if cfg!(windows) {
            assert!(path.ends_with(Path::new("llmwiki").join("bin").join("llmwiki.exe")));
        } else {
            assert!(path.ends_with(Path::new("llmwiki").join("bin").join("llmwiki")));
        }
    }
}
