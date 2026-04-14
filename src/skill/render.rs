use std::path::PathBuf;

use anyhow::Result;

use super::assets;
use super::{SkillHarness, SkillScope};

#[derive(Clone, Debug)]
pub struct RenderedSkillFile {
    pub relative_path: PathBuf,
    pub contents: Vec<u8>,
    pub executable: bool,
}

#[derive(Clone, Debug)]
pub struct RenderedSkillBundle {
    pub name: String,
    pub harness: SkillHarness,
    pub scope: SkillScope,
    pub files: Vec<RenderedSkillFile>,
}

pub fn render_bundle(harness: SkillHarness, scope: SkillScope) -> Result<RenderedSkillBundle> {
    let prefix = PathBuf::from("llm-wiki");
    let mut files = vec![
        RenderedSkillFile {
            relative_path: prefix.join("SKILL.md"),
            contents: assets::SKILL_MD.as_bytes().to_vec(),
            executable: false,
        },
        RenderedSkillFile {
            relative_path: prefix.join("scripts").join("llmwikiw.cmd"),
            contents: assets::WRAPPER_CMD.as_bytes().to_vec(),
            executable: false,
        },
        RenderedSkillFile {
            relative_path: prefix.join("scripts").join("llmwikiw.sh"),
            contents: assets::WRAPPER_SH.as_bytes().to_vec(),
            executable: true,
        },
    ];

    if matches!(harness, SkillHarness::Claude) {
        files.push(RenderedSkillFile {
            relative_path: prefix.join("templates").join(".gitkeep"),
            contents: Vec::new(),
            executable: false,
        });
    }

    Ok(RenderedSkillBundle {
        name: "llm-wiki".to_string(),
        harness,
        scope,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_bundle_contains_core_skill_files() {
        let bundle = render_bundle(SkillHarness::Claude, SkillScope::Repo).expect("bundle");

        let files = bundle
            .files
            .iter()
            .map(|file| file.relative_path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();
        assert!(files.contains(&"llm-wiki/SKILL.md".to_string()));
        assert!(files.contains(&"llm-wiki/scripts/llmwikiw.cmd".to_string()));
        assert!(files.contains(&"llm-wiki/scripts/llmwikiw.sh".to_string()));
        assert!(!files.contains(&"llm-wiki/README.md".to_string()));
    }
}
