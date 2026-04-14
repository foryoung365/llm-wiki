use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};

use crate::repo::Repo;
use crate::scan::{PageRecord, PageType, scan_repo};

pub fn rebuild(repo: &Repo) -> Result<usize> {
    let scan = scan_repo(repo)?;
    let contents = render_index(&scan.pages);
    fs::write(repo.index_file(), contents).context("写入 index.md 失败")?;
    Ok(scan.pages.len())
}

pub fn render_index(pages: &[PageRecord]) -> String {
    let mut grouped: BTreeMap<PageType, Vec<&PageRecord>> = BTreeMap::new();
    for page in pages {
        grouped
            .entry(page.page_type.clone())
            .or_default()
            .push(page);
    }

    for page_list in grouped.values_mut() {
        page_list.sort_by(|left, right| left.title.cmp(&right.title));
    }

    let mut out = String::from("# Index\n\n");
    for page_type in [
        PageType::Source,
        PageType::Entity,
        PageType::Concept,
        PageType::Question,
        PageType::Synthesis,
        PageType::Timeline,
    ] {
        out.push_str("## ");
        out.push_str(page_type.heading());
        out.push_str("\n");

        if let Some(page_list) = grouped.get(&page_type) {
            for page in page_list {
                let summary = if page.summary.trim().is_empty() {
                    "（暂无摘要）"
                } else {
                    page.summary.trim()
                };
                out.push_str("- [[");
                out.push_str(&page.wiki_key);
                out.push_str("]] - ");
                out.push_str(summary);
                out.push('\n');
            }
        }

        out.push('\n');
    }

    out
}
