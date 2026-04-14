use llmwiki::markdown::parse_markdown;

#[test]
fn parse_markdown_extracts_frontmatter_summary_and_links() {
    let contents = r#"---
page_type: entity
title: Large Language Model
slug: large-language-model
source_refs:
  - SRC-20260409-001
---

# Large Language Model

这是一个测试摘要。

## 相关页面
- [[concepts/transformer]]
"#;

    let parsed = parse_markdown("wiki/entities/llm.md", contents);
    let frontmatter = parsed.frontmatter.expect("frontmatter");

    assert_eq!(frontmatter.page_type.as_deref(), Some("entity"));
    assert_eq!(parsed.title.as_deref(), Some("Large Language Model"));
    assert_eq!(parsed.summary, "这是一个测试摘要。");
    assert_eq!(parsed.links, vec!["concepts/transformer".to_string()]);
}
