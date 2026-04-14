use std::mem;

use crate::repo::Repo;
use crate::scan::PageRecord;

#[derive(Clone, Debug)]
pub struct RankedPage<'a> {
    pub page: &'a PageRecord,
    pub score: usize,
}

pub fn rank_pages<'a>(query: &str, pages: &'a [PageRecord], limit: usize) -> Vec<RankedPage<'a>> {
    let normalized_query = query.trim().to_lowercase();
    let tokens = tokenize(&normalized_query);

    let mut ranked = pages
        .iter()
        .filter_map(|page| {
            let haystack_title = page.title.to_lowercase();
            let haystack_summary = page.summary.to_lowercase();
            let haystack_path = page.repo_path.as_str().to_lowercase();
            let mut score = 0usize;

            if !normalized_query.is_empty()
                && (haystack_title.contains(&normalized_query)
                    || haystack_summary.contains(&normalized_query)
                    || haystack_path.contains(&normalized_query))
            {
                score += 20;
            }

            for token in &tokens {
                if haystack_title.contains(token) {
                    score += 5;
                }
                if haystack_summary.contains(token) {
                    score += 3;
                }
                if haystack_path.contains(token) {
                    score += 2;
                }
            }

            (score > 0).then_some(RankedPage { page, score })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.page.title.cmp(&right.page.title))
    });
    ranked.truncate(limit);
    ranked
}

pub fn render_prepare_ingest_brief(
    repo: &Repo,
    raw_path: &str,
    source_id: &str,
    suggested_page: &str,
    related_pages: &[RankedPage<'_>],
) -> String {
    let mut out = String::new();
    out.push_str("# llmwiki prepare-ingest brief\n\n");
    out.push_str("目标来源: ");
    out.push_str(raw_path);
    out.push('\n');
    out.push_str("建议来源 ID: ");
    out.push_str(source_id);
    out.push('\n');
    out.push_str("建议来源页: ");
    out.push_str(suggested_page);
    out.push_str("\n\n");
    out.push_str("执行要求:\n");
    out.push_str("1. 先阅读 `AGENTS.md`、`wiki/_meta/index.md`、`wiki/_meta/log.md`\n");
    out.push_str("2. 读取指定原始来源，不得修改 `raw/`\n");
    out.push_str("3. 在 `wiki/sources/` 写入来源总结页，并提取候选实体、概念、未决问题\n");
    out.push_str("4. 更新相关 `wiki/entities/`、`wiki/concepts/`，必要时补写 `wiki/syntheses/` 或 `wiki/timelines/`\n");
    out.push_str("5. 完成后更新 `wiki/_meta/index.md` 与 `wiki/_meta/log.md`\n");
    out.push_str("6. 输出本次新建页、修改页、冲突项与待人工确认项\n\n");
    out.push_str("仓库根目录: ");
    out.push_str(repo.root().as_str());
    out.push_str("\n\n");

    if related_pages.is_empty() {
        out.push_str("建议先从 `wiki/_meta/index.md` 自行定位相关页面。\n");
    } else {
        out.push_str("优先审阅的相关页面:\n");
        for ranked in related_pages {
            out.push_str("- ");
            out.push_str(ranked.page.repo_path.as_str());
            out.push_str(" | ");
            out.push_str(&ranked.page.title);
            if !ranked.page.summary.trim().is_empty() {
                out.push_str(" | ");
                out.push_str(ranked.page.summary.trim());
            }
            out.push('\n');
        }
    }

    out
}

pub fn render_semantic_follow_up() -> String {
    [
        "请让 agent 继续执行语义 lint：",
        "1. 对照近期变更检查 contradictions",
        "2. 检查是否存在 stale claims 与需要提前 review 的页面",
        "3. 检查 orphan pages、missing cross-references、重要 gaps",
        "4. 对存在争议的页面并置来源，而不是私自抹平冲突",
    ]
    .join("\n")
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_word = String::new();
    let mut current_cjk = String::new();

    for ch in input.chars() {
        if is_cjk(ch) {
            flush_word(&mut current_word, &mut tokens);
            current_cjk.push(ch);
        } else {
            flush_cjk(&mut current_cjk, &mut tokens);

            if ch.is_alphanumeric() {
                for lowered in ch.to_lowercase() {
                    current_word.push(lowered);
                }
            } else {
                flush_word(&mut current_word, &mut tokens);
            }
        }
    }

    flush_word(&mut current_word, &mut tokens);
    flush_cjk(&mut current_cjk, &mut tokens);

    tokens.sort();
    tokens.dedup();
    tokens
}

fn flush_word(current: &mut String, tokens: &mut Vec<String>) {
    if current.chars().count() >= 2 {
        tokens.push(mem::take(current));
    } else {
        current.clear();
    }
}

fn flush_cjk(current: &mut String, tokens: &mut Vec<String>) {
    if current.is_empty() {
        return;
    }

    let chars = current.chars().collect::<Vec<_>>();
    if chars.len() >= 2 {
        let max_ngram = chars.len().min(4);
        for size in 2..=max_ngram {
            for start in 0..=chars.len() - size {
                tokens.push(chars[start..start + size].iter().collect());
            }
        }

        if chars.len() > max_ngram {
            tokens.push(chars.iter().collect());
        }
    }

    current.clear();
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x30000..=0x3134F
    )
}
