use anyhow::Result;

use crate::install;

pub fn run(force: bool) -> Result<()> {
    let summary = install::install_current_binary(force)?;
    println!("Installed shared CLI");
    println!("Path: {}", summary.path.display());
    println!("Source: {}", summary.source_path.display());
    println!(
        "Action: {}",
        match summary.action {
            install::InstallAction::Installed => "installed",
            install::InstallAction::Reused => "reused",
            install::InstallAction::Updated => "updated",
        }
    );
    Ok(())
}
