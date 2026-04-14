use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;

use crate::repo::Repo;
use crate::scan::Diagnostic;

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub heading: String,
    pub lines: Vec<String>,
}

pub fn append_entry(repo: &Repo, kind: &str, title: &str, lines: &[String]) -> Result<()> {
    ensure_log_exists(repo)?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(repo.log_file())
        .context("打开 log.md 失败")?;

    let mut chunk = String::new();
    chunk.push('\n');
    chunk.push_str("## [");
    chunk.push_str(&Local::now().format("%Y-%m-%d").to_string());
    chunk.push_str("] ");
    chunk.push_str(kind);
    chunk.push_str(" | ");
    chunk.push_str(title);
    chunk.push('\n');

    for line in lines {
        chunk.push_str("- ");
        chunk.push_str(line);
        chunk.push('\n');
    }

    file.write_all(chunk.as_bytes())
        .context("追加日志内容失败")?;
    Ok(())
}

pub fn read_recent(repo: &Repo, limit: usize) -> Result<Vec<LogEntry>> {
    ensure_log_exists(repo)?;
    let contents = fs::read_to_string(repo.log_file()).context("读取 log.md 失败")?;
    let mut entries = Vec::new();
    let mut current: Option<LogEntry> = None;

    for line in contents.lines() {
        if line.starts_with("## [") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(LogEntry {
                heading: line.to_string(),
                lines: Vec::new(),
            });
        } else if let Some(entry) = current.as_mut() {
            if !line.trim().is_empty() {
                entry.lines.push(line.to_string());
            }
        }
    }

    if let Some(entry) = current.take() {
        entries.push(entry);
    }

    let keep = limit.min(entries.len());
    Ok(entries.into_iter().rev().take(keep).collect())
}

pub fn validate_log_format(repo: &Repo) -> Result<Vec<Diagnostic>> {
    ensure_log_exists(repo)?;
    let contents = fs::read_to_string(repo.log_file()).context("读取 log.md 失败")?;
    let mut diagnostics = Vec::new();

    for (line_no, line) in contents.lines().enumerate() {
        if line.starts_with("## [") && !log_heading_regex().is_match(line) {
            diagnostics.push(Diagnostic::warning(
                "LOG001",
                repo.log_file().as_str(),
                &format!("第 {} 行日志标题格式不符合规范：{}", line_no + 1, line),
            ));
        }
    }

    Ok(diagnostics)
}

fn ensure_log_exists(repo: &Repo) -> Result<()> {
    if !repo.log_file().exists() {
        fs::write(repo.log_file(), "# Log\n").context("创建 log.md 失败")?;
    }
    Ok(())
}

fn log_heading_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^## \[\d{4}-\d{2}-\d{2}\] (init|ingest|query|lint) \| .+$")
            .expect("valid log heading regex")
    })
}
