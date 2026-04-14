use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use camino::{Utf8Path, Utf8PathBuf};

#[derive(Clone, Debug)]
pub struct Repo {
    root: Utf8PathBuf,
}

impl Repo {
    pub fn discover(explicit: Option<&str>) -> Result<Self> {
        let start = match explicit {
            Some(path) => utf8_from_std(resolve_fs_path(Path::new(path)))?,
            None => utf8_from_std(std::env::current_dir().context("无法读取当前目录")?)?,
        };

        let candidate = if start.is_file() {
            start.parent().map(Utf8Path::to_path_buf).unwrap_or(start)
        } else {
            start
        };

        let root = find_repo_root(&candidate).ok_or_else(|| {
            anyhow!("未找到 llm-wiki 仓库根目录；请在仓库内运行或使用 --repo 指定")
        })?;

        Ok(Self { root })
    }

    pub fn for_init(explicit: Option<&str>) -> Result<Self> {
        let root = match explicit {
            Some(path) => utf8_from_std(resolve_fs_path(Path::new(path)))?,
            None => utf8_from_std(std::env::current_dir().context("无法读取当前目录")?)?,
        };
        Ok(Self { root })
    }

    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn raw_dir(&self) -> Utf8PathBuf {
        self.root.join("raw")
    }

    pub fn wiki_dir(&self) -> Utf8PathBuf {
        self.root.join("wiki")
    }

    pub fn templates_dir(&self) -> Utf8PathBuf {
        self.root.join("templates")
    }

    pub fn docs_dir(&self) -> Utf8PathBuf {
        self.root.join("docs")
    }

    pub fn tools_dir(&self) -> Utf8PathBuf {
        self.root.join("tools")
    }

    pub fn plans_dir(&self) -> Utf8PathBuf {
        self.docs_dir().join("plans")
    }

    pub fn state_dir(&self) -> Utf8PathBuf {
        self.root.join("state")
    }

    pub fn meta_dir(&self) -> Utf8PathBuf {
        self.wiki_dir().join("_meta")
    }

    pub fn agents_file(&self) -> Utf8PathBuf {
        self.root.join("AGENTS.md")
    }

    pub fn index_file(&self) -> Utf8PathBuf {
        self.meta_dir().join("index.md")
    }

    pub fn log_file(&self) -> Utf8PathBuf {
        self.meta_dir().join("log.md")
    }

    pub fn resolve_input_path(&self, raw: &str) -> Result<Utf8PathBuf> {
        let candidate = Path::new(raw);
        let resolved = if candidate.is_absolute() {
            utf8_from_std(resolve_fs_path(candidate))?
        } else {
            self.root.join(raw)
        };

        if !resolved.starts_with(&self.root) {
            bail!("路径超出仓库范围：{}", resolved);
        }

        Ok(resolved)
    }

    pub fn relativize(&self, path: &Utf8Path) -> Result<Utf8PathBuf> {
        if let Ok(stripped) = path.strip_prefix(&self.root) {
            return Ok(Utf8PathBuf::from(stripped.as_str().replace('\\', "/")));
        }
        bail!("路径不在仓库内：{}", path);
    }

    pub fn ensure_layout(&self) -> Result<Vec<Utf8PathBuf>> {
        let mut created = Vec::new();

        for dir in [
            self.docs_dir(),
            self.plans_dir(),
            self.raw_dir(),
            self.raw_dir().join("inbox"),
            self.raw_dir().join("sources"),
            self.raw_dir().join("assets"),
            self.state_dir(),
            self.templates_dir(),
            self.wiki_dir(),
            self.wiki_dir().join("sources"),
            self.wiki_dir().join("entities"),
            self.wiki_dir().join("concepts"),
            self.wiki_dir().join("questions"),
            self.wiki_dir().join("syntheses"),
            self.wiki_dir().join("timelines"),
            self.meta_dir(),
        ] {
            ensure_dir(&dir, &mut created)?;
        }

        for dir in [
            self.raw_dir().join("inbox"),
            self.raw_dir().join("sources"),
            self.raw_dir().join("assets"),
            self.wiki_dir().join("sources"),
            self.wiki_dir().join("entities"),
            self.wiki_dir().join("concepts"),
            self.wiki_dir().join("questions"),
            self.wiki_dir().join("syntheses"),
            self.wiki_dir().join("timelines"),
        ] {
            ensure_file(&dir.join(".gitkeep"), "", &mut created)?;
        }

        ensure_file(
            &self.root.join(".gitignore"),
            include_str!("../.gitignore"),
            &mut created,
        )?;
        ensure_file(
            &self.root.join("README.md"),
            include_str!("../README.md"),
            &mut created,
        )?;
        ensure_file(
            &self.root.join("README.zh-CN.md"),
            include_str!("../README.zh-CN.md"),
            &mut created,
        )?;
        ensure_file(
            &self.root.join("README.en.md"),
            include_str!("../README.en.md"),
            &mut created,
        )?;
        ensure_file(
            &self.agents_file(),
            include_str!("../AGENTS.md"),
            &mut created,
        )?;
        ensure_file(
            &self.root.join("BACKLOG.csv"),
            include_str!("../BACKLOG.csv"),
            &mut created,
        )?;
        ensure_file(
            &self.docs_dir().join("ARCHITECTURE.md"),
            include_str!("../docs/ARCHITECTURE.md"),
            &mut created,
        )?;
        ensure_file(
            &self.docs_dir().join("EXECUTION_PLAN_zh.md"),
            include_str!("../docs/EXECUTION_PLAN_zh.md"),
            &mut created,
        )?;
        ensure_file(
            &self.state_dir().join("README.md"),
            include_str!("../state/README.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("README.md"),
            include_str!("../templates/README.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("source-summary.md"),
            include_str!("../templates/source-summary.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("entity.md"),
            include_str!("../templates/entity.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("concept.md"),
            include_str!("../templates/concept.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("question.md"),
            include_str!("../templates/question.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("synthesis.md"),
            include_str!("../templates/synthesis.md"),
            &mut created,
        )?;
        ensure_file(
            &self.templates_dir().join("timeline.md"),
            include_str!("../templates/timeline.md"),
            &mut created,
        )?;
        ensure_file(
            &self.index_file(),
            include_str!("../wiki/_meta/index.md"),
            &mut created,
        )?;
        ensure_file(&self.log_file(), "# Log\n", &mut created)?;

        Ok(created)
    }
}

fn find_repo_root(start: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut current = Some(start);

    while let Some(path) = current {
        if path.join("AGENTS.md").is_file()
            && path.join("raw").is_dir()
            && path.join("wiki").is_dir()
            && path.join("state").is_dir()
        {
            return Some(path.to_path_buf());
        }

        current = path.parent();
    }

    None
}

fn ensure_dir(path: &Utf8Path, created: &mut Vec<Utf8PathBuf>) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).with_context(|| format!("创建目录失败：{}", path))?;
        created.push(path.to_path_buf());
    }
    Ok(())
}

fn ensure_file(path: &Utf8Path, contents: &str, created: &mut Vec<Utf8PathBuf>) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("创建父目录失败：{}", parent))?;
        }
        fs::write(path, contents).with_context(|| format!("写入文件失败：{}", path))?;
        created.push(path.to_path_buf());
    }
    Ok(())
}

fn resolve_fs_path(path: &Path) -> PathBuf {
    if path.exists() {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

fn utf8_from_std(path: PathBuf) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).map_err(|p| anyhow!("路径不是有效 UTF-8：{}", p.display()))
}
