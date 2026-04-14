use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

use crate::scan::Diagnostic;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Frontmatter {
    pub page_type: Option<String>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub entity_refs: Vec<String>,
    #[serde(default)]
    pub concept_refs: Vec<String>,
    pub confidence: Option<String>,
    pub review_after: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ParsedMarkdown {
    pub frontmatter: Option<Frontmatter>,
    pub title: Option<String>,
    pub summary: String,
    pub body_text: String,
    pub links: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_markdown(path: &str, contents: &str) -> ParsedMarkdown {
    let (frontmatter, body, mut diagnostics) = extract_frontmatter(path, contents);
    let title = body
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned);
    let summary = extract_summary(&body);
    let links = wiki_link_regex()
        .captures_iter(contents)
        .filter_map(|caps| caps.get(1))
        .map(|capture| normalize_link(capture.as_str()))
        .filter(|link| !link.is_empty())
        .collect::<Vec<_>>();

    if title.is_none() {
        diagnostics.push(Diagnostic::warning(
            "PAGE001",
            path,
            "页面缺失 H1 标题，将退回 frontmatter.title 或文件名",
        ));
    }

    if body.trim().is_empty() {
        diagnostics.push(Diagnostic::warning("PAGE002", path, "页面正文为空"));
    }

    ParsedMarkdown {
        frontmatter,
        title,
        summary,
        body_text: body,
        links,
        diagnostics,
    }
}

fn extract_frontmatter(
    path: &str,
    contents: &str,
) -> (Option<Frontmatter>, String, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    if !contents.starts_with("---") {
        return (None, contents.to_string(), diagnostics);
    }

    let mut lines = contents.lines();
    let first = lines.next().unwrap_or_default();
    if first.trim() != "---" {
        return (None, contents.to_string(), diagnostics);
    }

    let mut yaml_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_yaml = true;

    for line in lines {
        if in_yaml && line.trim() == "---" {
            in_yaml = false;
            continue;
        }

        if in_yaml {
            yaml_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    if in_yaml {
        diagnostics.push(Diagnostic::warning(
            "FRM001",
            path,
            "frontmatter 未正常闭合，已按无 frontmatter 处理",
        ));
        return (None, contents.to_string(), diagnostics);
    }

    let yaml = yaml_lines.join("\n");
    let body = body_lines.join("\n");

    match serde_yaml::from_str::<Frontmatter>(&yaml) {
        Ok(frontmatter) => (Some(frontmatter), body, diagnostics),
        Err(error) => {
            diagnostics.push(Diagnostic::warning(
                "FRM002",
                path,
                &format!("frontmatter YAML 解析失败：{error}"),
            ));
            (None, body, diagnostics)
        }
    }
}

fn extract_summary(body: &str) -> String {
    let mut paragraph = Vec::new();
    let mut started = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if started {
                break;
            }
            continue;
        }

        if trimmed.starts_with('#') {
            if started {
                break;
            }
            continue;
        }

        paragraph.push(trimmed);
        started = true;
    }

    paragraph.join(" ")
}

fn normalize_link(raw: &str) -> String {
    raw.split('|')
        .next()
        .unwrap_or(raw)
        .split('#')
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_end_matches(".md")
        .replace('\\', "/")
}

fn wiki_link_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("valid wiki link regex"))
}
