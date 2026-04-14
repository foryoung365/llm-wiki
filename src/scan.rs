use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use chrono::NaiveDate;
use ignore::WalkBuilder;
use serde::Serialize;

use crate::markdown;
use crate::repo::Repo;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PageType {
    Source,
    Entity,
    Concept,
    Question,
    Synthesis,
    Timeline,
    Unknown,
}

impl PageType {
    pub fn from_str(value: &str) -> Self {
        match value {
            "source" => Self::Source,
            "entity" => Self::Entity,
            "concept" => Self::Concept,
            "question" => Self::Question,
            "synthesis" => Self::Synthesis,
            "timeline" => Self::Timeline,
            _ => Self::Unknown,
        }
    }

    pub fn from_path_key(key: &str) -> Self {
        match key.split('/').next().unwrap_or_default() {
            "sources" => Self::Source,
            "entities" => Self::Entity,
            "concepts" => Self::Concept,
            "questions" => Self::Question,
            "syntheses" => Self::Synthesis,
            "timelines" => Self::Timeline,
            _ => Self::Unknown,
        }
    }

    pub fn heading(&self) -> &'static str {
        match self {
            Self::Source => "Sources",
            Self::Entity => "Entities",
            Self::Concept => "Concepts",
            Self::Question => "Questions",
            Self::Synthesis => "Syntheses",
            Self::Timeline => "Timelines",
            Self::Unknown => "Other",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
}

impl Diagnostic {
    pub fn warning(code: &str, path: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            severity: "warning".to_string(),
            path: path.to_string(),
            message: message.to_string(),
        }
    }

    pub fn error(code: &str, path: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            severity: "error".to_string(),
            path: path.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PageRecord {
    pub repo_path: Utf8PathBuf,
    pub wiki_key: String,
    pub page_type: PageType,
    pub title: String,
    pub summary: String,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub confidence: Option<String>,
    pub review_after: Option<NaiveDate>,
    pub source_refs: Vec<String>,
    pub entity_refs: Vec<String>,
    pub concept_refs: Vec<String>,
    pub outbound_links: Vec<String>,
    pub inbound_count: usize,
    pub has_h1: bool,
    pub has_body: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanResult {
    pub pages: Vec<PageRecord>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn scan_repo(repo: &Repo) -> Result<ScanResult> {
    let mut pages = Vec::new();
    let mut diagnostics = Vec::new();

    let mut walker = WalkBuilder::new(repo.wiki_dir());
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .ignore(true);

    for entry in walker.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(Diagnostic::warning(
                    "SCAN001",
                    "wiki",
                    &format!("扫描目录时发生错误：{error}"),
                ));
                continue;
            }
        };

        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        let path = match Utf8PathBuf::from_path_buf(entry.path().to_path_buf()) {
            Ok(path) => path,
            Err(path) => {
                diagnostics.push(Diagnostic::warning(
                    "SCAN002",
                    "wiki",
                    &format!("跳过非 UTF-8 路径：{}", path.display()),
                ));
                continue;
            }
        };

        if path.extension() != Some("md") {
            continue;
        }

        if path.starts_with(repo.meta_dir()) {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("读取 Markdown 文件失败：{}", path))?;
        let repo_path = repo.relativize(&path)?;
        let path_label = repo_path.as_str().to_string();
        let parsed = markdown::parse_markdown(&path_label, &contents);
        diagnostics.extend(parsed.diagnostics);

        let wiki_relative = path
            .strip_prefix(repo.wiki_dir())
            .expect("page lives under wiki dir");
        let wiki_key = wiki_relative.with_extension("").as_str().replace('\\', "/");
        let frontmatter = parsed.frontmatter.unwrap_or_default();
        let page_type = frontmatter
            .page_type
            .as_deref()
            .map(PageType::from_str)
            .unwrap_or_else(|| PageType::from_path_key(&wiki_key));
        let has_h1 = parsed.title.is_some();
        let title = parsed
            .title
            .or(frontmatter.title.clone())
            .unwrap_or_else(|| {
                repo_path
                    .file_stem()
                    .map(str::to_string)
                    .unwrap_or_else(|| "untitled".to_string())
            });
        let review_after = frontmatter
            .review_after
            .as_deref()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());

        pages.push(PageRecord {
            repo_path,
            wiki_key,
            page_type,
            title,
            summary: parsed.summary,
            slug: frontmatter.slug,
            status: frontmatter.status,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
            confidence: frontmatter.confidence,
            review_after,
            source_refs: dedupe(frontmatter.source_refs),
            entity_refs: dedupe(frontmatter.entity_refs),
            concept_refs: dedupe(frontmatter.concept_refs),
            outbound_links: dedupe(parsed.links),
            inbound_count: 0,
            has_h1,
            has_body: !parsed.body_text.trim().is_empty(),
        });
    }

    let key_to_index = pages
        .iter()
        .enumerate()
        .map(|(index, page)| (page.wiki_key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut inbound_counts = vec![0usize; pages.len()];

    for page in &pages {
        let mut seen = BTreeSet::new();
        for link in &page.outbound_links {
            if !seen.insert(link) {
                continue;
            }
            if let Some(index) = key_to_index.get(link) {
                inbound_counts[*index] += 1;
            }
        }
    }

    for (page, inbound_count) in pages.iter_mut().zip(inbound_counts.into_iter()) {
        page.inbound_count = inbound_count;
    }

    Ok(ScanResult { pages, diagnostics })
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}
