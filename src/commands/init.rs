use std::io::{self, Write};

use anyhow::Result;

use crate::repo::Repo;
use crate::skill::install::{self, SkillInstallRequest};
use crate::skill::{SkillHarness, SkillScope};

pub fn run(repo: &Repo, install_skills: &[SkillHarness]) -> Result<()> {
    let created = repo.ensure_layout()?;
    println!("Initialized repository at {}", repo.root());

    if created.is_empty() {
        println!("No files were created; existing layout already satisfies the starter structure.");
    } else {
        println!("Created {} paths:", created.len());
        for path in created {
            println!("- {}", path);
        }
    }

    if install_skills.is_empty() {
        println!(
            "Next step: run `llmwiki skill install --harness codex --scope repo` if you want to attach a harness skill."
        );
        return Ok(());
    }

    println!("Installed skills:");
    for harness in install_skills {
        println!("- {}: installing shared CLI and skill files...", harness);
        io::stdout().flush()?;
        let summary = install::install_skill(
            Some(repo),
            SkillInstallRequest {
                harness: *harness,
                scope: SkillScope::Repo,
                force: false,
            },
        )?;
        println!(
            "- {} -> {} (CLI: {})",
            harness,
            summary.skill_dir.display(),
            summary.cli.path.display()
        );
    }

    Ok(())
}
