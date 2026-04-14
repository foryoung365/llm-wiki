use anyhow::Result;

use crate::convert::{self, ConvertRequest};
use crate::repo::Repo;

pub fn run(repo: &Repo, request: ConvertRequest<'_>) -> Result<()> {
    let summary = convert::run(repo, request)?;
    println!("Bundle written to {}", summary.bundle_dir);
    println!("Platform: {}", summary.platform);
    println!("Assets: {}", summary.assets);
    if summary.warnings == 0 {
        println!("Warnings: none");
    } else {
        println!("Warnings: {}", summary.warnings);
    }
    Ok(())
}
