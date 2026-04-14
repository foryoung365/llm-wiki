use chrono::NaiveDate;

pub fn next_source_id<'a>(existing: impl IntoIterator<Item = &'a str>, date: NaiveDate) -> String {
    let prefix = format!("SRC-{}", date.format("%Y%m%d"));
    let next = existing
        .into_iter()
        .filter(|candidate| candidate.starts_with(&prefix))
        .filter_map(|candidate| candidate.rsplit('-').next())
        .filter_map(|suffix| suffix.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;

    format!("{prefix}-{next:03}")
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;

    for ch in input.chars() {
        if ch.is_alphanumeric() {
            for lowered in ch.to_lowercase() {
                out.push(lowered);
            }
            previous_dash = false;
        } else if !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }

    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed.to_string()
    }
}
