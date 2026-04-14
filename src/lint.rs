use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;
use serde::Serialize;

use crate::logbook;
use crate::prompt;
use crate::repo::Repo;
use crate::scan::{Diagnostic, ScanResult, scan_repo};
use crate::state;

#[derive(Clone, Debug, Serialize)]
pub struct LintSummary {
    pub contradictions: usize,
    pub broken_links: usize,
    pub duplicate_titles: usize,
    pub stale_pages: usize,
    pub orphan_pages: usize,
    pub gaps: usize,
    pub missing_index_entries: usize,
    pub log_format_issues: usize,
    pub parser_diagnostics: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct LintReport {
    pub generated_at: String,
    pub summary: LintSummary,
    pub diagnostics: Vec<Diagnostic>,
    pub semantic_follow_up: String,
}

pub fn run(repo: &Repo, append_log: bool) -> Result<LintReport> {
    let scan = scan_repo(repo)?;
    let report = build_report(repo, &scan)?;

    fs::create_dir_all(repo.state_dir()).context("创建 state/ 目录失败")?;
    fs::write(
        repo.state_dir().join("lint-latest.json"),
        serde_json::to_string_pretty(&report).context("序列化 lint 报告失败")?,
    )
    .context("写入 lint-latest.json 失败")?;

    state::sync_from_scan(repo, &scan)?;

    if append_log {
        let lines = vec![
            format!("contradictions: {}", report.summary.contradictions),
            format!("stale: {}", report.summary.stale_pages),
            format!("orphans: {}", report.summary.orphan_pages),
            format!("gaps: {}", report.summary.gaps),
            format!("mechanical_diagnostics: {}", report.diagnostics.len()),
        ];
        logbook::append_entry(repo, "lint", "mechanical-health-check", &lines)?;
    }

    Ok(report)
}

pub fn render_console(report: &LintReport) -> String {
    let mut out = String::new();
    out.push_str("Lint summary\n");
    out.push_str(&format!(
        "- contradictions: {}\n- broken_links: {}\n- duplicate_titles: {}\n- stale_pages: {}\n- orphan_pages: {}\n- gaps: {}\n- missing_index_entries: {}\n- log_format_issues: {}\n- parser_diagnostics: {}\n",
        report.summary.contradictions,
        report.summary.broken_links,
        report.summary.duplicate_titles,
        report.summary.stale_pages,
        report.summary.orphan_pages,
        report.summary.gaps,
        report.summary.missing_index_entries,
        report.summary.log_format_issues,
        report.summary.parser_diagnostics,
    ));

    if report.diagnostics.is_empty() {
        out.push_str("\nNo mechanical issues detected.\n");
    } else {
        out.push_str("\nMechanical diagnostics:\n");
        for diagnostic in &report.diagnostics {
            out.push_str("- ");
            out.push_str(&diagnostic.code);
            out.push_str(" | ");
            out.push_str(&diagnostic.path);
            out.push_str(" | ");
            out.push_str(&diagnostic.message);
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str(&report.semantic_follow_up);
    out.push('\n');
    out
}

fn build_report(repo: &Repo, scan: &ScanResult) -> Result<LintReport> {
    let mut diagnostics = scan.diagnostics.clone();
    let today = Local::now().date_naive();
    let key_set = scan
        .pages
        .iter()
        .map(|page| page.wiki_key.as_str())
        .collect::<BTreeSet<_>>();

    let mut duplicate_titles = 0usize;
    let mut contradictions = 0usize;
    let mut broken_links = 0usize;
    let mut stale_pages = 0usize;
    let mut orphan_pages = 0usize;
    let mut missing_targets = BTreeSet::new();
    let mut missing_index_entries = 0usize;

    let mut titles: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for page in &scan.pages {
        titles
            .entry(page.title.as_str())
            .or_default()
            .push(page.repo_path.as_str());

        if page.status.as_deref() == Some("disputed") {
            contradictions += 1;
        }

        if let Some(review_after) = page.review_after
            && review_after < today
        {
            stale_pages += 1;
            diagnostics.push(Diagnostic::warning(
                "STALE001",
                page.repo_path.as_str(),
                &format!("review_after 已过期：{}", review_after),
            ));
        }

        if page.inbound_count == 0 {
            orphan_pages += 1;
            diagnostics.push(Diagnostic::warning(
                "ORPHAN001",
                page.repo_path.as_str(),
                "页面没有任何入链",
            ));
        }

        for link in &page.outbound_links {
            if !key_set.contains(link.as_str()) {
                broken_links += 1;
                missing_targets.insert(link.clone());
                diagnostics.push(Diagnostic::error(
                    "LINK001",
                    page.repo_path.as_str(),
                    &format!("断裂 wiki link：[[{}]]", link),
                ));
            }
        }
    }

    for (title, paths) in titles {
        if paths.len() > 1 {
            duplicate_titles += 1;
            let joined = paths.join(", ");
            diagnostics.push(Diagnostic::warning(
                "TITLE001",
                &joined,
                &format!("发现重复标题：{}", title),
            ));
        }
    }

    let index_links = parse_index_links(repo)?;
    for page in &scan.pages {
        if !index_links.contains(page.wiki_key.as_str()) {
            missing_index_entries += 1;
            diagnostics.push(Diagnostic::warning(
                "IDX001",
                repo.index_file().as_str(),
                &format!("index.md 未覆盖页面：{}", page.wiki_key),
            ));
        }
    }

    let log_diagnostics = logbook::validate_log_format(repo)?;
    let log_format_issues = log_diagnostics.len();
    diagnostics.extend(log_diagnostics);

    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then(left.path.cmp(&right.path))
            .then(left.message.cmp(&right.message))
    });

    Ok(LintReport {
        generated_at: Local::now().to_rfc3339(),
        summary: LintSummary {
            contradictions,
            broken_links,
            duplicate_titles,
            stale_pages,
            orphan_pages,
            gaps: missing_targets.len(),
            missing_index_entries,
            log_format_issues,
            parser_diagnostics: scan.diagnostics.len(),
        },
        diagnostics,
        semantic_follow_up: prompt::render_semantic_follow_up(),
    })
}

fn parse_index_links(repo: &Repo) -> Result<BTreeSet<String>> {
    if !repo.index_file().exists() {
        return Ok(BTreeSet::new());
    }

    let contents = fs::read_to_string(repo.index_file()).context("读取 index.md 失败")?;
    let links = wiki_link_regex()
        .captures_iter(&contents)
        .filter_map(|caps| caps.get(1))
        .map(|capture| capture.as_str().trim().trim_end_matches(".md").to_string())
        .collect::<BTreeSet<_>>();
    Ok(links)
}

fn wiki_link_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("valid regex"))
}
