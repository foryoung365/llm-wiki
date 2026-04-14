use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::commands;
use crate::repo::Repo;
use crate::skill::{SkillHarness, SkillScope};

#[derive(Debug, Parser)]
#[command(name = "llmwiki")]
#[command(
    about = "维护 llm-wiki 仓库的确定性 CLI",
    long_about = "维护 llm-wiki 仓库的确定性 CLI。\n\n它负责初始化仓库、安装共享 CLI 与 harness skill、转换来源、重建索引与状态，以及执行 lint；语义检索、跨页综合与高价值答案写回仍由 agent 按 AGENTS.md 完成。\n\n常见起步顺序：\n  1. llmwiki init\n  2. llmwiki install\n  3. llmwiki skill install --harness codex --scope repo\n\n可使用 `llmwiki doctor` 检查转换环境，使用 `llmwiki skill doctor` 检查共享 CLI 与 skill 安装状态。"
)]
pub struct Cli {
    #[arg(long, global = true, help = "显式指定 llm-wiki 仓库根目录")]
    pub repo: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "初始化 llm-wiki 仓库骨架")]
    Init {
        #[arg(help = "初始化目标目录；缺省为当前目录")]
        path: Option<String>,
        #[arg(
            long = "install-skill",
            value_enum,
            help = "初始化完成后，顺带安装一个或多个 harness skill"
        )]
        install_skills: Vec<SkillHarness>,
    },
    #[command(about = "把当前 llmwiki 安装到共享路径，供 skill 复用")]
    Install {
        #[arg(long, help = "覆盖共享路径中的现有 CLI")]
        force: bool,
    },
    #[command(about = "安装或诊断多 harness skill")]
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    #[command(about = "把 URL 或本地文件转换为 raw/inbox 下的 Markdown bundle")]
    Convert {
        #[arg(help = "输入 URL 或本地文件路径")]
        input: String,
        #[arg(long, help = "覆盖输出目录；缺省写入 raw/inbox/<slug>/")]
        output: Option<String>,
        #[arg(long, help = "抓取网页时使用的 User-Agent")]
        user_agent: Option<String>,
        #[arg(long, help = "抓取网页时使用的 Cookie 头")]
        cookie_header: Option<String>,
        #[arg(long, help = "视频页额外下载媒体文件，而不仅是元数据与字幕")]
        with_media: bool,
    },
    #[command(about = "检查 convert 所需环境、目录与 sidecar")]
    Doctor,
    #[command(name = "install-sidecar", about = "安装可选 sidecar；当前支持 yt-dlp")]
    InstallSidecar {
        #[arg(value_enum, help = "sidecar 名称")]
        sidecar: SidecarName,
        #[arg(long, help = "覆盖仓库内现有 sidecar")]
        force: bool,
    },
    #[command(name = "sync-state", about = "重建 state/ 下的派生状态文件")]
    SyncState,
    #[command(name = "rebuild-index", about = "重建 wiki/_meta/index.md")]
    RebuildIndex,
    #[command(about = "查看最近日志条目")]
    Recent {
        #[arg(long, default_value_t = 10, help = "输出条目数量")]
        limit: usize,
    },
    #[command(about = "按页面类型列出 wiki 条目")]
    List {
        #[arg(long, value_enum, help = "仅输出指定页面类型")]
        page_type: Option<PageTypeFilter>,
    },
    #[command(
        name = "prepare-ingest",
        about = "为 agent 生成 ingest brief，而不直接执行 ingest"
    )]
    PrepareIngest {
        #[arg(help = "raw/ 下的来源路径或待处理输入路径")]
        raw_path: String,
    },
    #[command(about = "执行 wiki 结构与语义健康检查")]
    Lint {
        #[arg(long, help = "仅输出检查结果，不向 wiki/_meta/log.md 追加条目")]
        no_log: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    #[command(
        about = "安装指定 harness 的薄 skill，并确保共享 CLI 已可用",
        long_about = "安装指定 harness 的薄 skill，并确保共享 CLI 已可用。\n\nrepo 级常用目标：\n  - Claude: .claude/skills/llm-wiki\n  - OpenCode: .opencode/skills/llm-wiki\n  - OpenClaw: skills/llm-wiki\n  - Codex: .agents/skills/llm-wiki"
    )]
    Install {
        #[arg(long, value_enum, help = "目标 harness")]
        harness: SkillHarness,
        #[arg(
            long,
            value_enum,
            default_value_t = SkillScope::Repo,
            help = "安装作用域：repo 写入当前仓库，user 写入用户目录"
        )]
        scope: SkillScope,
        #[arg(long, help = "覆盖现有 skill 文件与共享 CLI")]
        force: bool,
    },
    #[command(
        about = "诊断共享 CLI 与 skill 目录是否完整",
        long_about = "诊断共享 CLI 与 skill 目录是否完整。\n\n该命令会按 wrapper 的实际顺序检查 LLMWIKI_BIN、共享安装路径与 PATH，并验证目标 skill 目录下的 SKILL.md 与 wrapper 脚本是否齐备。"
    )]
    Doctor {
        #[arg(long, value_enum, help = "仅检查指定 harness；缺省检查全部 harness")]
        harness: Option<SkillHarness>,
        #[arg(
            long,
            value_enum,
            default_value_t = SkillScope::Repo,
            help = "诊断作用域：repo 检查当前仓库，user 检查用户目录"
        )]
        scope: SkillScope,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageTypeFilter {
    Source,
    Entity,
    Concept,
    Question,
    Synthesis,
    Timeline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SidecarName {
    #[value(name = "yt-dlp")]
    YtDlp,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    dispatch(cli)
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init {
            path,
            install_skills,
        } => {
            let repo = Repo::for_init(cli.repo.as_deref().or(path.as_deref()))?;
            commands::init::run(&repo, &install_skills)
        }
        Command::Install { force } => commands::install::run(force),
        Command::Skill { command } => match command {
            SkillCommand::Install {
                harness,
                scope,
                force,
            } => {
                let repo = match scope {
                    SkillScope::Repo => Some(Repo::discover(cli.repo.as_deref())?),
                    SkillScope::User => None,
                };
                commands::skill::run_install(repo.as_ref(), harness, scope, force)
            }
            SkillCommand::Doctor { harness, scope } => {
                let repo = match scope {
                    SkillScope::Repo => Some(Repo::discover(cli.repo.as_deref())?),
                    SkillScope::User => None,
                };
                commands::skill::run_doctor(repo.as_ref(), harness, scope)
            }
        },
        command => {
            let repo = Repo::discover(cli.repo.as_deref())?;
            match command {
                Command::Convert {
                    input,
                    output,
                    user_agent,
                    cookie_header,
                    with_media,
                } => commands::convert::run(
                    &repo,
                    crate::convert::ConvertRequest {
                        input: &input,
                        output: output.as_deref(),
                        user_agent: user_agent.as_deref(),
                        cookie_header: cookie_header.as_deref(),
                        with_media,
                    },
                ),
                Command::Doctor => commands::doctor::run(&repo),
                Command::Install { .. } | Command::Skill { .. } => {
                    unreachable!("install and skill handled above")
                }
                Command::InstallSidecar { sidecar, force } => match sidecar {
                    SidecarName::YtDlp => {
                        commands::install_sidecar::run_install_yt_dlp(&repo, force)
                    }
                },
                Command::SyncState => commands::sync_state::run(&repo),
                Command::RebuildIndex => commands::rebuild_index::run(&repo),
                Command::Recent { limit } => commands::recent::run(&repo, limit),
                Command::List { page_type } => commands::list::run(&repo, page_type),
                Command::PrepareIngest { raw_path } => {
                    commands::prepare_ingest::run(&repo, &raw_path)
                }
                Command::Lint { no_log } => commands::lint::run(&repo, !no_log),
                Command::Init { .. } => unreachable!("init handled above"),
            }
        }
    }
}
