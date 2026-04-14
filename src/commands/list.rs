use anyhow::Result;

use crate::cli::PageTypeFilter;
use crate::repo::Repo;
use crate::scan::{PageType, scan_repo};

pub fn run(repo: &Repo, filter: Option<PageTypeFilter>) -> Result<()> {
    let scan = scan_repo(repo)?;
    let requested = filter.map(map_filter);

    for page in scan.pages {
        if requested
            .as_ref()
            .is_some_and(|kind| kind != &page.page_type)
        {
            continue;
        }

        println!(
            "[{}] {} | {}",
            page.page_type.heading(),
            page.repo_path,
            page.title
        );
    }

    Ok(())
}

fn map_filter(filter: PageTypeFilter) -> PageType {
    match filter {
        PageTypeFilter::Source => PageType::Source,
        PageTypeFilter::Entity => PageType::Entity,
        PageTypeFilter::Concept => PageType::Concept,
        PageTypeFilter::Question => PageType::Question,
        PageTypeFilter::Synthesis => PageType::Synthesis,
        PageTypeFilter::Timeline => PageType::Timeline,
    }
}
