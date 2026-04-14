use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use reqwest::blocking::Client;

use crate::repo::Repo;

const INSTALL_USER_AGENT: &str = "llmwiki/0.1";
const YT_DLP_ENV_VAR: &str = "LLMWIKI_YT_DLP";
const YT_DLP_RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryOrigin {
    EnvVar(&'static str),
    RepoLocal,
    Path,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedBinary {
    pub path: PathBuf,
    pub origin: BinaryOrigin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallSummary {
    pub path: PathBuf,
    pub version: String,
    pub download_url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct YtDlpReleaseSpec {
    platform_dir: &'static str,
    download_name: &'static str,
    installed_name: &'static str,
}

pub fn resolve_yt_dlp(repo: &Repo) -> Option<ResolvedBinary> {
    resolve_yt_dlp_with(repo, env::var_os(YT_DLP_ENV_VAR), env::var_os("PATH"))
}

pub fn install_yt_dlp(repo: &Repo, force: bool) -> Result<InstallSummary> {
    let spec = current_yt_dlp_release_spec()?;
    let target = repo_local_yt_dlp_primary_path(repo, spec);
    if target.exists() && !force {
        let version = probe_version(&target).with_context(|| {
            format!(
                "existing yt-dlp sidecar is present but failed verification: {}",
                target.display()
            )
        })?;
        return Ok(InstallSummary {
            path: target,
            version,
            download_url: yt_dlp_download_url(spec),
        });
    }

    let parent = target.parent().ok_or_else(|| {
        anyhow!(
            "failed to determine parent directory for yt-dlp sidecar: {}",
            target.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create sidecar directory: {}", parent.display()))?;

    let client = Client::builder()
        .user_agent(INSTALL_USER_AGENT)
        .build()
        .context("failed to build sidecar installer client")?;
    let download_url = yt_dlp_download_url(spec);
    let bytes = client
        .get(&download_url)
        .send()
        .with_context(|| format!("failed to download yt-dlp from {download_url}"))?
        .error_for_status()
        .with_context(|| format!("yt-dlp download failed: {download_url}"))?
        .bytes()
        .context("failed to read yt-dlp download response")?;
    if bytes.is_empty() {
        bail!("downloaded yt-dlp sidecar is empty");
    }

    let temp_target = target.with_extension(format!(
        "{}.download",
        target
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin")
    ));
    fs::write(&temp_target, bytes.as_ref()).with_context(|| {
        format!(
            "failed to write temporary yt-dlp sidecar: {}",
            temp_target.display()
        )
    })?;
    mark_executable(&temp_target)?;
    fs::rename(&temp_target, &target).with_context(|| {
        format!(
            "failed to move yt-dlp sidecar into place: {}",
            target.display()
        )
    })?;
    mark_executable(&target)?;

    let version = probe_version(&target).with_context(|| {
        format!(
            "downloaded yt-dlp sidecar failed verification: {}",
            target.display()
        )
    })?;

    Ok(InstallSummary {
        path: target,
        version,
        download_url,
    })
}

pub fn repo_local_yt_dlp_dir(repo: &Repo) -> Result<PathBuf> {
    let spec = current_yt_dlp_release_spec()?;
    Ok(repo
        .root()
        .join("tools/yt-dlp")
        .join(spec.platform_dir)
        .into())
}

fn resolve_yt_dlp_with(
    repo: &Repo,
    env_override: Option<OsString>,
    path_env: Option<OsString>,
) -> Option<ResolvedBinary> {
    if let Some(path) = env_override
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Some(ResolvedBinary {
            path,
            origin: BinaryOrigin::EnvVar(YT_DLP_ENV_VAR),
        });
    }

    if let Some(path) = find_repo_local_yt_dlp(repo) {
        return Some(ResolvedBinary {
            path,
            origin: BinaryOrigin::RepoLocal,
        });
    }

    find_on_path("yt-dlp", path_env).map(|path| ResolvedBinary {
        path,
        origin: BinaryOrigin::Path,
    })
}

fn find_repo_local_yt_dlp(repo: &Repo) -> Option<PathBuf> {
    let dir = repo_local_yt_dlp_dir(repo).ok()?;
    candidate_binary_names("yt-dlp")
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

fn repo_local_yt_dlp_primary_path(repo: &Repo, spec: YtDlpReleaseSpec) -> PathBuf {
    repo.root()
        .join("tools/yt-dlp")
        .join(spec.platform_dir)
        .join(spec.installed_name)
        .into()
}

fn yt_dlp_download_url(spec: YtDlpReleaseSpec) -> String {
    format!("{YT_DLP_RELEASE_BASE}/{}", spec.download_name)
}

fn current_yt_dlp_release_spec() -> Result<YtDlpReleaseSpec> {
    match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => Ok(YtDlpReleaseSpec {
            platform_dir: "windows-x86_64",
            download_name: "yt-dlp.exe",
            installed_name: "yt-dlp.exe",
        }),
        ("windows", "x86") => Ok(YtDlpReleaseSpec {
            platform_dir: "windows-x86",
            download_name: "yt-dlp_x86.exe",
            installed_name: "yt-dlp.exe",
        }),
        ("windows", "aarch64") => Ok(YtDlpReleaseSpec {
            platform_dir: "windows-aarch64",
            download_name: "yt-dlp_arm64.exe",
            installed_name: "yt-dlp.exe",
        }),
        ("linux", "x86_64") => Ok(YtDlpReleaseSpec {
            platform_dir: "linux-x86_64",
            download_name: "yt-dlp_linux",
            installed_name: "yt-dlp",
        }),
        ("linux", "aarch64") => Ok(YtDlpReleaseSpec {
            platform_dir: "linux-aarch64",
            download_name: "yt-dlp_linux_aarch64",
            installed_name: "yt-dlp",
        }),
        ("macos", _) => Ok(YtDlpReleaseSpec {
            platform_dir: "macos-universal",
            download_name: "yt-dlp_macos",
            installed_name: "yt-dlp",
        }),
        (os, arch) => bail!("yt-dlp sidecar is not packaged for the current platform: {os}/{arch}"),
    }
}

fn probe_version(path: &Path) -> Result<String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to execute sidecar: {}", path.display()))?;
    if !output.status.success() {
        bail!(
            "sidecar returned a non-success status: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        bail!("sidecar produced an empty version string");
    }
    Ok(version)
}

fn find_on_path(binary: &str, path_env: Option<OsString>) -> Option<PathBuf> {
    let path_env = path_env?;
    let names = candidate_binary_names(binary);
    env::split_paths(&path_env)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

fn candidate_binary_names(binary: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            format!("{binary}.exe"),
            format!("{binary}.cmd"),
            format!("{binary}.bat"),
            binary.to_string(),
        ]
    } else {
        vec![binary.to_string()]
    }
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
            .with_context(|| format!("failed to mark sidecar executable: {}", path.display()))?;
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
    fn repo_local_dir_uses_platform_layout() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo = Repo::for_init(root.path().to_str()).expect("repo");

        let dir = repo_local_yt_dlp_dir(&repo).expect("repo local dir");

        assert!(dir.ends_with(expected_platform_dir()));
    }

    #[test]
    fn resolve_yt_dlp_prefers_repo_local_over_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo = Repo::for_init(root.path().to_str()).expect("repo");
        let repo_local_dir = repo_local_yt_dlp_dir(&repo).expect("repo local dir");
        fs::create_dir_all(&repo_local_dir).expect("create repo local dir");
        let repo_local = repo_local_dir.join(candidate_binary_names("yt-dlp")[0].as_str());
        fs::write(&repo_local, b"stub").expect("write repo local stub");

        let path_dir = tempfile::tempdir().expect("path tempdir");
        let path_stub = path_dir
            .path()
            .join(candidate_binary_names("yt-dlp")[0].as_str());
        fs::write(&path_stub, b"stub").expect("write path stub");

        let resolved = resolve_yt_dlp_with(
            &repo,
            None,
            Some(path_dir.path().as_os_str().to_os_string()),
        )
        .expect("resolved");

        assert_eq!(resolved.origin, BinaryOrigin::RepoLocal);
        assert_eq!(resolved.path, repo_local);
    }

    fn expected_platform_dir() -> &'static str {
        match (env::consts::OS, env::consts::ARCH) {
            ("windows", "x86_64") => "windows-x86_64",
            ("windows", "x86") => "windows-x86",
            ("windows", "aarch64") => "windows-aarch64",
            ("linux", "x86_64") => "linux-x86_64",
            ("linux", "aarch64") => "linux-aarch64",
            ("macos", _) => "macos-universal",
            _ => "unsupported",
        }
    }
}
