use anyhow::{Result, bail};
use chrono::Local;

use crate::prompt;
use crate::repo::Repo;
use crate::scan::scan_repo;
use crate::source_id::{next_source_id, slugify};

pub fn run(repo: &Repo, raw_path: &str) -> Result<()> {
    let resolved = repo.resolve_input_path(raw_path)?;
    if !resolved.exists() {
        bail!("来源文件不存在：{}", resolved);
    }
    if !resolved.starts_with(repo.raw_dir()) {
        bail!("prepare-ingest 仅允许处理 raw/ 目录下的来源：{}", resolved);
    }

    let relative = repo.relativize(&resolved)?;
    let scan = scan_repo(repo)?;
    let existing_ids = scan
        .pages
        .iter()
        .flat_map(|page| page.source_refs.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let source_id = next_source_id(existing_ids, Local::now().date_naive());
    let file_stem = resolved.file_stem().unwrap_or("source");
    let suggested_page = format!("wiki/sources/{}-{}.md", source_id, slugify(file_stem));
    let ranked = prompt::rank_pages(file_stem, &scan.pages, 6);
    let brief = prompt::render_prepare_ingest_brief(
        repo,
        relative.as_str(),
        &source_id,
        &suggested_page,
        &ranked,
    );
    println!("{}", brief);
    Ok(())
}
