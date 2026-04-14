use anyhow::Result;

use crate::repo::Repo;
use crate::sidecar;

pub fn run_install_yt_dlp(repo: &Repo, force: bool) -> Result<()> {
    let summary = sidecar::install_yt_dlp(repo, force)?;
    println!("Installed sidecar: yt-dlp");
    println!("Path: {}", summary.path.display());
    println!("Version: {}", summary.version);
    println!("Source: {}", summary.download_url);
    Ok(())
}
