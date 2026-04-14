pub mod cli;
pub mod commands;
pub mod convert;
pub mod index;
pub mod install;
pub mod lint;
pub mod logbook;
pub mod markdown;
pub mod prompt;
pub mod repo;
pub mod scan;
pub mod sidecar;
pub mod skill;
pub mod source_id;
pub mod state;

pub fn run() -> anyhow::Result<()> {
    cli::run()
}
