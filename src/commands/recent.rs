use anyhow::Result;

use crate::logbook;
use crate::repo::Repo;

pub fn run(repo: &Repo, limit: usize) -> Result<()> {
    let entries = logbook::read_recent(repo, limit)?;
    if entries.is_empty() {
        println!("No log entries found.");
        return Ok(());
    }

    for entry in entries {
        println!("{}", entry.heading);
        for line in entry.lines {
            println!("{}", line);
        }
        println!();
    }

    Ok(())
}
