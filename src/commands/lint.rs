use anyhow::Result;

use crate::lint;
use crate::repo::Repo;

pub fn run(repo: &Repo, append_log: bool) -> Result<()> {
    let report = lint::run(repo, append_log)?;
    println!("{}", lint::render_console(&report));
    println!("Wrote {}", repo.state_dir().join("lint-latest.json"));
    Ok(())
}
