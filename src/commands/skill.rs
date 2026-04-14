use std::io::{self, Write};

use anyhow::Result;

use crate::repo::Repo;
use crate::skill::doctor;
use crate::skill::install::{self, SkillInstallRequest};
use crate::skill::{SkillHarness, SkillScope};

pub fn run_install(
    repo: Option<&Repo>,
    harness: SkillHarness,
    scope: SkillScope,
    force: bool,
) -> Result<()> {
    println!("Installing {harness} skill for {scope} scope...");
    io::stdout().flush()?;
    let summary = install::install_skill(
        repo,
        SkillInstallRequest {
            harness,
            scope,
            force,
        },
    )?;
    println!("Installed skill");
    println!("Harness: {}", summary.harness);
    println!("Scope: {}", summary.scope);
    println!("Skill path: {}", summary.skill_dir.display());
    println!("CLI path: {}", summary.cli.path.display());
    println!(
        "CLI action: {}",
        install::describe_install_action(summary.cli.action)
    );
    Ok(())
}

pub fn run_doctor(
    repo: Option<&Repo>,
    harness: Option<SkillHarness>,
    scope: SkillScope,
) -> Result<()> {
    let report = doctor::run(repo, harness, scope);
    println!("Skill doctor");
    for check in report.checks {
        let status = match check.status {
            doctor::DoctorStatus::Ok => "ok",
            doctor::DoctorStatus::Warn => "warn",
        };
        println!("- {} [{}] {}", check.name, status, check.detail);
    }
    Ok(())
}
