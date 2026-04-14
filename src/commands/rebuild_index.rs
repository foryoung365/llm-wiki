use anyhow::Result;

use crate::index;
use crate::repo::Repo;

pub fn run(repo: &Repo) -> Result<()> {
    let count = index::rebuild(repo)?;
    println!(
        "Rebuilt {} with {} indexed wiki pages",
        repo.index_file(),
        count
    );
    Ok(())
}
