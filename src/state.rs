use std::fs;

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;

use crate::repo::Repo;
use crate::scan::{PageRecord, ScanResult, scan_repo};

#[derive(Clone, Debug, Serialize)]
pub struct SourceManifestEntry {
    pub source_id: String,
    pub title: String,
    pub page_path: String,
    pub status: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PageGraphNode {
    pub key: String,
    pub path: String,
    pub page_type: String,
    pub title: String,
    pub summary: String,
    pub status: Option<String>,
    pub source_refs: Vec<String>,
    pub inbound_count: usize,
    pub outbound_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PageGraphLink {
    pub from: String,
    pub to: String,
    pub broken: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PageGraph {
    pub generated_at: String,
    pub pages: Vec<PageGraphNode>,
    pub links: Vec<PageGraphLink>,
    pub diagnostics: Vec<crate::scan::Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct SyncSummary {
    pub pages: usize,
    pub sources: usize,
    pub diagnostics: usize,
}

pub fn sync(repo: &Repo) -> Result<SyncSummary> {
    let scan = scan_repo(repo)?;
    sync_from_scan(repo, &scan)
}

pub fn sync_from_scan(repo: &Repo, scan: &ScanResult) -> Result<SyncSummary> {
    fs::create_dir_all(repo.state_dir()).context("创建 state/ 目录失败")?;

    let source_manifest = build_source_manifest(&scan.pages);
    let source_manifest_text = source_manifest
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .context("序列化 source manifest 失败")?
        .join("\n");
    let mut source_manifest_text = source_manifest_text;
    if !source_manifest_text.is_empty() {
        source_manifest_text.push('\n');
    }
    fs::write(
        repo.state_dir().join("source_manifest.jsonl"),
        source_manifest_text,
    )
    .context("写入 source_manifest.jsonl 失败")?;

    let graph = build_page_graph(scan);
    fs::write(
        repo.state_dir().join("page_graph.json"),
        serde_json::to_string_pretty(&graph).context("序列化 page_graph.json 失败")?,
    )
    .context("写入 page_graph.json 失败")?;

    Ok(SyncSummary {
        pages: scan.pages.len(),
        sources: source_manifest.len(),
        diagnostics: scan.diagnostics.len(),
    })
}

fn build_source_manifest(pages: &[PageRecord]) -> Vec<SourceManifestEntry> {
    let mut entries = pages
        .iter()
        .filter(|page| matches!(page.page_type, crate::scan::PageType::Source))
        .map(|page| SourceManifestEntry {
            source_id: page
                .source_refs
                .first()
                .cloned()
                .unwrap_or_else(|| infer_source_id(page)),
            title: page.title.clone(),
            page_path: page.repo_path.as_str().to_string(),
            status: page.status.clone(),
            updated_at: page.updated_at.clone(),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    entries
}

fn build_page_graph(scan: &ScanResult) -> PageGraph {
    let keys = scan
        .pages
        .iter()
        .map(|page| page.wiki_key.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut links = Vec::new();

    for page in &scan.pages {
        for link in &page.outbound_links {
            links.push(PageGraphLink {
                from: page.wiki_key.clone(),
                to: link.clone(),
                broken: !keys.contains(link.as_str()),
            });
        }
    }
    links.sort_by(|left, right| left.from.cmp(&right.from).then(left.to.cmp(&right.to)));

    let mut pages = scan
        .pages
        .iter()
        .map(|page| PageGraphNode {
            key: page.wiki_key.clone(),
            path: page.repo_path.as_str().to_string(),
            page_type: format!("{:?}", page.page_type).to_lowercase(),
            title: page.title.clone(),
            summary: page.summary.clone(),
            status: page.status.clone(),
            source_refs: page.source_refs.clone(),
            inbound_count: page.inbound_count,
            outbound_count: page.outbound_links.len(),
        })
        .collect::<Vec<_>>();
    pages.sort_by(|left, right| left.key.cmp(&right.key));

    PageGraph {
        generated_at: Local::now().to_rfc3339(),
        pages,
        links,
        diagnostics: scan.diagnostics.clone(),
    }
}

fn infer_source_id(page: &PageRecord) -> String {
    page.repo_path
        .file_stem()
        .unwrap_or("unknown")
        .split('-')
        .take(3)
        .collect::<Vec<_>>()
        .join("-")
}
