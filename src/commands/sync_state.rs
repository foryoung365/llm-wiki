use anyhow::Result;

use crate::repo::Repo;
use crate::state;

pub fn run(repo: &Repo) -> Result<()> {
    let summary = state::sync(repo)?;
    println!(
        "State synchronized: {} pages, {} source entries, {} diagnostics",
        summary.pages, summary.sources, summary.diagnostics
    );
    println!("- {}", repo.state_dir().join("source_manifest.jsonl"));
    println!("- {}", repo.state_dir().join("page_graph.json"));
    Ok(())
}
