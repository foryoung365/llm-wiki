pub mod assets;
pub mod doctor;
pub mod install;
pub mod render;
pub mod targets;

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SkillHarness {
    Claude,
    Opencode,
    Openclaw,
    Codex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SkillScope {
    Repo,
    User,
}

impl std::fmt::Display for SkillHarness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SkillHarness::Claude => "claude",
            SkillHarness::Opencode => "opencode",
            SkillHarness::Openclaw => "openclaw",
            SkillHarness::Codex => "codex",
        };
        f.write_str(value)
    }
}

impl std::fmt::Display for SkillScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SkillScope::Repo => "repo",
            SkillScope::User => "user",
        };
        f.write_str(value)
    }
}

pub const ALL_HARNESSES: [SkillHarness; 4] = [
    SkillHarness::Claude,
    SkillHarness::Opencode,
    SkillHarness::Openclaw,
    SkillHarness::Codex,
];
