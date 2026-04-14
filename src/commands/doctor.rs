use anyhow::Result;

use crate::convert;
use crate::repo::Repo;

pub fn run(repo: &Repo) -> Result<()> {
    let summary = convert::doctor(repo);

    println!("Convert doctor");
    for check in summary.checks {
        let status = match check.status {
            convert::DoctorStatus::Ok => "ok",
            convert::DoctorStatus::Warn => "warn",
        };
        println!("- {} [{}] {}", check.name, status, check.detail);
    }
    println!("Hint: run `llmwiki skill doctor` for harness and shared CLI checks.");

    Ok(())
}
