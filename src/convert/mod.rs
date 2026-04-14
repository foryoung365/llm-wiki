use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use calamine::{Data, Reader, open_workbook_auto};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use comrak::{Options as ComrakOptions, markdown_to_commonmark};
use html_to_markdown_rs::convert as html_to_markdown;
use readabilityrs::{Readability, ReadabilityOptions};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use scraper::{Html, Selector};
use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::repo::Repo;
use crate::sidecar::{self, BinaryOrigin};
use crate::source_id::slugify;

const DEFAULT_USER_AGENT: &str = "llmwiki/0.1";
const QUALITY_FALLBACK_THRESHOLD: f64 = 0.80;

pub struct ConvertRequest<'a> {
    pub input: &'a str,
    pub output: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub cookie_header: Option<&'a str>,
    pub with_media: bool,
}

#[derive(Clone, Debug)]
pub struct ConvertSummary {
    pub bundle_dir: Utf8PathBuf,
    pub platform: String,
    pub assets: usize,
    pub warnings: usize,
}

#[derive(Clone, Debug)]
pub struct DoctorSummary {
    pub checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoctorStatus {
    Ok,
    Warn,
}

#[derive(Clone, Debug)]
enum SourceInput {
    Url(Url),
    File(PathBuf),
}

#[derive(Clone, Debug)]
struct PreparedBundle {
    title: Option<String>,
    source_url: Option<String>,
    source_path: Option<String>,
    source_type: String,
    platform: String,
    converter_chain: Vec<String>,
    fidelity: String,
    warnings: Vec<String>,
    markdown: String,
    mime: Option<String>,
    assets: Vec<BundleFile>,
    source_files: Vec<BundleFile>,
}

#[derive(Clone, Debug)]
struct BundleFile {
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct AssetCandidate {
    raw: String,
    resolved: Url,
}

#[derive(Clone, Debug, Serialize)]
struct NoteFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    source_type: String,
    platform: String,
    converter_chain: Vec<String>,
    captured_at: String,
    fidelity: String,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AssetMetadata {
    path: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
    bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
struct BundleMetadata {
    title: Option<String>,
    source_url: Option<String>,
    source_path: Option<String>,
    source_type: String,
    platform: String,
    converter_chain: Vec<String>,
    captured_at: String,
    fidelity: String,
    warnings: Vec<String>,
    mime: Option<String>,
    assets: Vec<AssetMetadata>,
}

pub fn run(repo: &Repo, request: ConvertRequest<'_>) -> Result<ConvertSummary> {
    let input = resolve_input(repo, request.input)?;
    let bundle_slug = default_bundle_slug(&input);
    let bundle_dir = resolve_bundle_dir(repo, request.output, &bundle_slug)?;
    ensure_bundle_target(&bundle_dir)?;

    fs::create_dir_all(bundle_dir.join("assets"))
        .with_context(|| format!("failed to create assets dir: {}", bundle_dir.join("assets")))?;
    fs::create_dir_all(bundle_dir.join("source"))
        .with_context(|| format!("failed to create source dir: {}", bundle_dir.join("source")))?;

    let client = build_http_client(request.user_agent, request.cookie_header)?;
    let prepared = match &input {
        SourceInput::Url(url) => convert_url(repo, url, &bundle_dir, &client, &request),
        SourceInput::File(path) => convert_file(path),
    }?;

    let assets = write_bundle(&bundle_dir, &prepared)?;

    Ok(ConvertSummary {
        bundle_dir,
        platform: prepared.platform,
        assets,
        warnings: prepared.warnings.len(),
    })
}

pub fn doctor(repo: &Repo) -> DoctorSummary {
    let mut checks = Vec::new();
    let inbox = repo.raw_dir().join("inbox");
    checks.push(DoctorCheck {
        name: "raw/inbox".to_string(),
        status: if inbox.exists() {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        detail: if inbox.exists() {
            format!("available at {}", inbox)
        } else {
            format!("missing: {}", inbox)
        },
    });

    checks.push(probe_yt_dlp(repo));
    checks.push(probe_binary(
        "wechat-article-to-markdown",
        &["--help"],
        "optional fallback for weixin articles",
    ));

    DoctorSummary { checks }
}

fn resolve_input(repo: &Repo, raw: &str) -> Result<SourceInput> {
    if let Ok(url) = Url::parse(raw) {
        if matches!(url.scheme(), "http" | "https") {
            return Ok(SourceInput::Url(url));
        }
    }

    let candidate = Path::new(raw);
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        let repo_relative = repo.root().join(raw);
        if repo_relative.exists() {
            PathBuf::from(repo_relative.as_str())
        } else {
            std::env::current_dir()
                .context("鏃犳硶璇诲彇褰撳墠鐩綍")?
                .join(candidate)
        }
    };

    if !path.exists() {
        bail!("杈撳叆涓嶅瓨鍦細{}", path.display());
    }
    if path.is_dir() {
        bail!("directory input is not supported: {}", path.display());
    }

    Ok(SourceInput::File(
        fs::canonicalize(&path).unwrap_or(path.to_path_buf()),
    ))
}

fn default_bundle_slug(input: &SourceInput) -> String {
    match input {
        SourceInput::Url(url) => {
            let stem = url
                .path_segments()
                .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
                .unwrap_or(url.host_str().unwrap_or("web"));
            slugify(stem)
        }
        SourceInput::File(path) => slugify(
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("source"),
        ),
    }
}

fn resolve_bundle_dir(
    repo: &Repo,
    explicit: Option<&str>,
    default_slug: &str,
) -> Result<Utf8PathBuf> {
    match explicit {
        Some(path) => repo.resolve_input_path(path),
        None => Ok(repo.raw_dir().join("inbox").join(default_slug)),
    }
}

fn ensure_bundle_target(bundle_dir: &Utf8Path) -> Result<()> {
    if bundle_dir.exists() {
        bail!("bundle 鐩綍宸插瓨鍦細{}", bundle_dir);
    }
    Ok(())
}

#[allow(dead_code)]
fn build_client(user_agent: Option<&str>, cookie_header: Option<&str>) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(user_agent.unwrap_or(DEFAULT_USER_AGENT))
            .context("User-Agent 鏃犳晥")?,
    );
    if let Some(cookie_header) = cookie_header {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(cookie_header).context("Cookie Header 鏃犳晥")?,
        );
    }

    Client::builder()
        .default_headers(headers)
        .build()
        .context("鍒涘缓 HTTP Client 澶辫触")
}

fn build_http_client(user_agent: Option<&str>, cookie_header: Option<&str>) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(user_agent.unwrap_or(DEFAULT_USER_AGENT))
            .context("invalid User-Agent")?,
    );
    if let Some(cookie_header) = cookie_header {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(cookie_header).context("invalid Cookie header")?,
        );
    }

    Client::builder()
        .default_headers(headers)
        .build()
        .context("failed to build HTTP client")
}

fn probe_binary(binary: &str, args: &[&str], missing_detail: &str) -> DoctorCheck {
    match Command::new(binary).args(args).output() {
        Ok(output) if output.status.success() => {
            let detail = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if detail.is_empty() {
                format!("available via `{binary}`")
            } else {
                detail
            };
            DoctorCheck {
                name: binary.to_string(),
                status: DoctorStatus::Ok,
                detail,
            }
        }
        Ok(output) => DoctorCheck {
            name: binary.to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "present but returned a non-success status: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        },
        Err(error) => DoctorCheck {
            name: binary.to_string(),
            status: DoctorStatus::Warn,
            detail: format!("{missing_detail}; probe failed: {error}"),
        },
    }
}

fn probe_yt_dlp(repo: &Repo) -> DoctorCheck {
    let Some(resolved) = sidecar::resolve_yt_dlp(repo) else {
        return DoctorCheck {
            name: "yt-dlp".to_string(),
            status: DoctorStatus::Warn,
            detail: "required for bilibili/douyin video URLs; install with `llmwiki install-sidecar yt-dlp`, set `LLMWIKI_YT_DLP`, or add `yt-dlp` to PATH".to_string(),
        };
    };

    match Command::new(&resolved.path).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let version = if version.is_empty() {
                "available".to_string()
            } else {
                version
            };
            DoctorCheck {
                name: "yt-dlp".to_string(),
                status: DoctorStatus::Ok,
                detail: format!(
                    "{} via {} ({})",
                    version,
                    describe_binary_origin(&resolved.origin),
                    resolved.path.display()
                ),
            }
        }
        Ok(output) => DoctorCheck {
            name: "yt-dlp".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "resolved at {} via {}, but returned a non-success status: {}",
                resolved.path.display(),
                describe_binary_origin(&resolved.origin),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        },
        Err(error) => DoctorCheck {
            name: "yt-dlp".to_string(),
            status: DoctorStatus::Warn,
            detail: format!(
                "resolved at {} via {}, but probe failed: {}",
                resolved.path.display(),
                describe_binary_origin(&resolved.origin),
                error
            ),
        },
    }
}

fn describe_binary_origin(origin: &BinaryOrigin) -> String {
    match origin {
        BinaryOrigin::EnvVar(name) => format!("env:{name}"),
        BinaryOrigin::RepoLocal => "repo-local sidecar".to_string(),
        BinaryOrigin::Path => "PATH".to_string(),
    }
}

fn convert_url(
    repo: &Repo,
    url: &Url,
    bundle_dir: &Utf8Path,
    client: &Client,
    request: &ConvertRequest<'_>,
) -> Result<PreparedBundle> {
    match detect_url_platform(url) {
        "bilibili" | "douyin" => convert_video_url(repo, url, bundle_dir, request),
        "weixin" => match convert_weixin_url(url, client) {
            Ok(bundle) => Ok(bundle),
            Err(error) => match convert_weixin_with_sidecar(url) {
                Ok(mut bundle) => {
                    bundle.warnings.push(format!(
                        "weixin adapter failed; used wechat-article-to-markdown fallback: {error}"
                    ));
                    Ok(bundle)
                }
                Err(sidecar_error) => {
                    let mut fallback = convert_generic_web(url, client, "weixin")?;
                    fallback
                        .warnings
                        .push(format!("weixin adapter failed: {error}"));
                    fallback.warnings.push(format!(
                        "wechat-article-to-markdown fallback unavailable: {sidecar_error}"
                    ));
                    Ok(fallback)
                }
            },
        },
        platform => convert_generic_web(url, client, platform),
    }
}

fn convert_file(path: &Path) -> Result<PreparedBundle> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut bundle = match extension.as_str() {
        "pdf" => convert_pdf_file(path)?,
        "docx" | "pptx" | "xlsx" => convert_ooxml_file(path)?,
        "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => convert_spreadsheet_file(path)?,
        "md" | "markdown" => convert_markdown_file(path)?,
        "txt" => convert_text_file(path)?,
        "html" | "htm" => convert_html_file(path)?,
        "json" => convert_structured_text_file(path, "json")?,
        "xml" => convert_structured_text_file(path, "xml")?,
        _ => {
            if guess_plain_text(path)? {
                convert_text_file(path)?
            } else {
                bail!(
                    "鏆備笉鏀寔鐨勬枃浠剁被鍨嬶細{}",
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("(鏃犳墿灞曞悕)")
                );
            }
        }
    };

    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("original");
    bundle.source_path = Some(path.display().to_string());
    bundle.source_files.push(BundleFile {
        relative_path: format!("source/{}", filename),
        bytes: fs::read(path).unwrap_or_default(),
    });
    if bundle.title.is_none() {
        bundle.title = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(ToOwned::to_owned);
    }
    if bundle.mime.is_none() {
        bundle.mime = mime_from_extension(path.extension().and_then(|ext| ext.to_str()));
    }

    Ok(bundle)
}

fn convert_pdf_file(path: &Path) -> Result<PreparedBundle> {
    let document = unpdf::parse_file(path)
        .with_context(|| format!("failed to parse PDF: {}", path.display()))?;
    let options = unpdf::render::RenderOptions::new()
        .with_image_dir("assets")
        .with_image_prefix("assets/")
        .with_cleanup_preset(unpdf::CleanupPreset::Standard);
    let markdown =
        unpdf::render::to_markdown(&document, &options).context("PDF 杞?Markdown 澶辫触")?;
    let json = unpdf::render::to_json(&document, unpdf::JsonFormat::Pretty)
        .context("PDF 杞?JSON 澶辫触")?;

    let mut assets = Vec::new();
    for (resource_id, resource) in &document.resources {
        let filename = normalize_asset_filename(
            resource.filename.as_deref(),
            resource_id,
            Some(&resource.mime_type),
        );
        assets.push(BundleFile {
            relative_path: format!("assets/{}", filename),
            bytes: resource.data.clone(),
        });
    }

    Ok(PreparedBundle {
        title: document.metadata.title.clone(),
        source_url: None,
        source_path: None,
        source_type: "file".to_string(),
        platform: "pdf".to_string(),
        converter_chain: vec!["unpdf".to_string(), "comrak".to_string()],
        fidelity: "structured".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&markdown),
        mime: Some("application/pdf".to_string()),
        assets,
        source_files: vec![BundleFile {
            relative_path: "source/unpdf.json".to_string(),
            bytes: json.into_bytes(),
        }],
    })
}

fn convert_ooxml_file(path: &Path) -> Result<PreparedBundle> {
    let document = undoc::parse_file(path)
        .with_context(|| format!("failed to parse Office file: {}", path.display()))?;
    let options = undoc::render::RenderOptions::new()
        .with_image_dir("assets")
        .with_image_prefix("assets/");
    let markdown =
        undoc::render::to_markdown(&document, &options).context("Office 杞?Markdown 澶辫触")?;
    let json = undoc::render::to_json(&document, undoc::render::JsonFormat::Pretty)
        .context("Office 杞?JSON 澶辫触")?;

    let mut assets = Vec::new();
    for (resource_id, resource) in &document.resources {
        let suggested = resource.suggested_filename(resource_id);
        let filename = normalize_asset_filename(
            Some(suggested.as_str()),
            resource_id,
            resource.mime_type.as_deref(),
        );
        assets.push(BundleFile {
            relative_path: format!("assets/{}", filename),
            bytes: resource.data.clone(),
        });
    }

    Ok(PreparedBundle {
        title: document.metadata.title.clone(),
        source_url: None,
        source_path: None,
        source_type: "file".to_string(),
        platform: path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase(),
        converter_chain: vec!["undoc".to_string(), "comrak".to_string()],
        fidelity: "structured".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&markdown),
        mime: mime_from_extension(path.extension().and_then(|ext| ext.to_str())),
        assets,
        source_files: vec![BundleFile {
            relative_path: "source/undoc.json".to_string(),
            bytes: json.into_bytes(),
        }],
    })
}

fn convert_spreadsheet_file(path: &Path) -> Result<PreparedBundle> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("failed to open spreadsheet: {}", path.display()))?;
    let mut markdown = String::new();

    for sheet_name in workbook.sheet_names().to_owned() {
        let range = workbook
            .worksheet_range(&sheet_name)
            .with_context(|| format!("璇诲彇宸ヤ綔琛ㄥけ璐ワ細{sheet_name}"))?;
        if range.is_empty() {
            continue;
        }

        if !markdown.is_empty() {
            markdown.push_str("\n\n");
        }
        markdown.push_str(&format!("## {}\n\n", sheet_name));
        markdown.push_str(&render_range_as_markdown(&range.rows().collect::<Vec<_>>()));
    }

    if markdown.trim().is_empty() {
        markdown.push_str("_empty workbook_\n");
    }

    Ok(PreparedBundle {
        title: path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(ToOwned::to_owned),
        source_url: None,
        source_path: None,
        source_type: "file".to_string(),
        platform: path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("xls")
            .to_ascii_lowercase(),
        converter_chain: vec!["calamine".to_string(), "comrak".to_string()],
        fidelity: "structured".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&markdown),
        mime: mime_from_extension(path.extension().and_then(|ext| ext.to_str())),
        assets: Vec::new(),
        source_files: Vec::new(),
    })
}

fn convert_markdown_file(path: &Path) -> Result<PreparedBundle> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read file: {}", path.display()))?;
    Ok(PreparedBundle {
        title: extract_heading(&contents).or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        }),
        source_url: None,
        source_path: None,
        source_type: "file".to_string(),
        platform: "markdown".to_string(),
        converter_chain: vec!["native".to_string(), "comrak".to_string()],
        fidelity: "structured".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&contents),
        mime: Some("text/markdown".to_string()),
        assets: Vec::new(),
        source_files: Vec::new(),
    })
}

fn convert_text_file(path: &Path) -> Result<PreparedBundle> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read file: {}", path.display()))?;
    let title = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned);
    let body = if let Some(title) = &title {
        format!("# {}\n\n{}", title, contents.trim())
    } else {
        contents
    };

    Ok(PreparedBundle {
        title,
        source_url: None,
        source_path: None,
        source_type: "file".to_string(),
        platform: "text".to_string(),
        converter_chain: vec!["native".to_string(), "comrak".to_string()],
        fidelity: "plain".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&body),
        mime: Some("text/plain".to_string()),
        assets: Vec::new(),
        source_files: Vec::new(),
    })
}

fn convert_html_file(path: &Path) -> Result<PreparedBundle> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read HTML file: {}", path.display()))?;
    convert_html_bytes(&bytes, None, "generic", None)
}

fn convert_structured_text_file(path: &Path, kind: &str) -> Result<PreparedBundle> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read file: {}", path.display()))?;
    let title = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned);
    let language = if kind == "json" { "json" } else { "xml" };
    let body = if let Some(title) = &title {
        format!("# {}\n\n```{}\n{}\n```", title, language, contents.trim())
    } else {
        format!("```{}\n{}\n```", language, contents.trim())
    };

    Ok(PreparedBundle {
        title,
        source_url: None,
        source_path: None,
        source_type: "file".to_string(),
        platform: kind.to_string(),
        converter_chain: vec!["native".to_string(), "comrak".to_string()],
        fidelity: "plain".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&body),
        mime: mime_from_extension(Some(kind)),
        assets: Vec::new(),
        source_files: Vec::new(),
    })
}

fn convert_generic_web(url: &Url, client: &Client, platform: &str) -> Result<PreparedBundle> {
    let response = client
        .get(url.clone())
        .send()
        .with_context(|| format!("failed to fetch page: {url}"))?
        .error_for_status()
        .with_context(|| format!("page returned non-success status: {url}"))?;

    let final_url = response.url().clone();
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let bytes = response.bytes().context("璇诲彇缃戦〉鍐呭澶辫触")?;

    let mut bundle = convert_html_bytes(&bytes, Some(&final_url), platform, Some(client))?;
    bundle.source_url = Some(final_url.to_string());
    bundle.mime = mime.or(Some("text/html".to_string()));
    bundle.source_files.push(BundleFile {
        relative_path: "source/original.html".to_string(),
        bytes: bytes.to_vec(),
    });
    Ok(bundle)
}

fn convert_weixin_url(url: &Url, client: &Client) -> Result<PreparedBundle> {
    let response = client
        .get(url.clone())
        .send()
        .with_context(|| format!("failed to fetch weixin page: {url}"))?
        .error_for_status()
        .with_context(|| format!("weixin page returned non-success status: {url}"))?;
    let final_url = response.url().clone();
    let bytes = response
        .bytes()
        .context("璇诲彇鍏紬鍙锋枃绔犲唴瀹瑰け璐?")?;
    let html = String::from_utf8_lossy(&bytes).into_owned();

    let document = Html::parse_document(&html);
    let title = select_first_text(&document, &["h1.rich_media_title", "#activity-name"])
        .filter(|value| !value.is_empty());
    let account_name =
        select_first_text(&document, &["#js_name"]).filter(|value| !value.is_empty());
    let published_at =
        select_first_text(&document, &["#publish_time"]).filter(|value| !value.is_empty());
    let content = select_first_html(&document, &["#js_content", ".rich_media_content"])
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("鏈壘鍒板叕浼楀彿姝ｆ枃瀹瑰櫒"))?;

    let converted =
        html_to_markdown(&content, None).context("鍏紬鍙锋枃绔?HTML 杞?Markdown 澶辫触")?;
    let mut warnings = converted
        .warnings
        .into_iter()
        .map(|warning| warning.message)
        .collect::<Vec<_>>();

    let image_candidates = collect_image_candidates(&content, &final_url);
    let (assets, replacements, asset_warnings) = download_remote_assets(client, &image_candidates)?;
    warnings.extend(asset_warnings);

    let mut markdown = converted.content.unwrap_or_default();
    apply_replacements(&mut markdown, &replacements);

    let mut prefix = String::new();
    if let Some(account_name) = &account_name {
        prefix.push_str(&format!("> Account: {}\n", account_name));
    }
    if let Some(published_at) = &published_at {
        prefix.push_str(&format!("> Published: {}\n", published_at));
    }
    if !prefix.is_empty() {
        prefix.push('\n');
    }
    markdown = format!("{}{}", prefix, markdown.trim());

    Ok(normalize_bundle_markdown(PreparedBundle {
        title,
        source_url: Some(final_url.to_string()),
        source_path: None,
        source_type: "url".to_string(),
        platform: "weixin".to_string(),
        converter_chain: vec![
            "reqwest".to_string(),
            "weixin-adapter".to_string(),
            "html-to-markdown-rs".to_string(),
            "comrak".to_string(),
        ],
        fidelity: "structured".to_string(),
        warnings,
        markdown: ensure_title_heading(
            markdown,
            select_first_text(&document, &["h1.rich_media_title", "#activity-name"]),
        ),
        mime: Some("text/html".to_string()),
        assets,
        source_files: vec![BundleFile {
            relative_path: "source/original.html".to_string(),
            bytes: bytes.to_vec(),
        }],
    }))
}

fn convert_weixin_with_sidecar(url: &Url) -> Result<PreparedBundle> {
    let work_dir = create_temp_work_dir("wechat-sidecar")?;
    let result = run_weixin_sidecar_command("wechat-article-to-markdown", url, &work_dir);
    let cleanup = fs::remove_dir_all(&work_dir);
    if let Err(error) = cleanup {
        let _ = error;
    }
    result
}

fn run_weixin_sidecar_command(program: &str, url: &Url, work_dir: &Path) -> Result<PreparedBundle> {
    let output = Command::new(program)
        .arg(url.as_str())
        .current_dir(work_dir)
        .output()
        .map_err(|error| {
            anyhow!(
                "failed to run wechat-article-to-markdown: {error}. install it to enable the optional weixin fallback"
            )
        })?;
    if !output.status.success() {
        bail!(
            "wechat-article-to-markdown returned non-success status: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let sidecar_output = work_dir.join("output");
    let article_dir = fs::read_dir(&sidecar_output)
        .with_context(|| {
            format!(
                "failed to read sidecar output dir: {}",
                sidecar_output.display()
            )
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .ok_or_else(|| anyhow!("wechat sidecar did not produce an article directory"))?;

    let markdown_path = fs::read_dir(&article_dir)
        .with_context(|| format!("failed to read article dir: {}", article_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .ok_or_else(|| anyhow!("wechat sidecar did not emit a markdown file"))?;
    let markdown = fs::read_to_string(&markdown_path).with_context(|| {
        format!(
            "failed to read sidecar markdown: {}",
            markdown_path.display()
        )
    })?;

    let mut assets = Vec::new();
    let images_dir = article_dir.join("images");
    if images_dir.exists() {
        for entry in fs::read_dir(&images_dir).with_context(|| {
            format!(
                "failed to read sidecar images dir: {}",
                images_dir.display()
            )
        })? {
            let entry = entry.context("failed to inspect sidecar image entry")?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("image.bin");
            assets.push(BundleFile {
                relative_path: format!("assets/{}", filename),
                bytes: fs::read(&path).unwrap_or_default(),
            });
        }
    }

    Ok(PreparedBundle {
        title: extract_heading(&markdown).or_else(|| {
            article_dir
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        }),
        source_url: Some(url.to_string()),
        source_path: None,
        source_type: "url".to_string(),
        platform: "weixin".to_string(),
        converter_chain: vec![
            "wechat-article-to-markdown".to_string(),
            "comrak".to_string(),
        ],
        fidelity: "structured".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&markdown),
        mime: Some("text/html".to_string()),
        assets,
        source_files: vec![BundleFile {
            relative_path: "source/wechat-article-to-markdown.md".to_string(),
            bytes: markdown.into_bytes(),
        }],
    })
}

fn create_temp_work_dir(label: &str) -> Result<PathBuf> {
    let timestamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000);
    let dir = std::env::temp_dir().join(format!(
        "llmwiki-{label}-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create temp work dir: {}", dir.display()))?;
    Ok(dir)
}

fn convert_html_bytes(
    bytes: &[u8],
    url: Option<&Url>,
    platform: &str,
    client: Option<&Client>,
) -> Result<PreparedBundle> {
    let html = String::from_utf8_lossy(bytes).into_owned();
    let mut options = rs_trafilatura::Options {
        include_images: true,
        include_links: true,
        output_markdown: true,
        ..Default::default()
    };
    if let Some(url) = url {
        options.url = Some(url.to_string());
    }

    let extracted = rs_trafilatura::extract_bytes_with_options(bytes, &options)
        .context("rs-trafilatura 鎶藉彇澶辫触")?;
    let mut converter_chain = vec!["rs-trafilatura".to_string()];
    let mut warnings = extracted.warnings.clone();
    let mut title = extracted.metadata.title.clone();
    let mut fidelity = if extracted.extraction_quality >= QUALITY_FALLBACK_THRESHOLD {
        "structured".to_string()
    } else {
        "semi".to_string()
    };
    let mut markdown = extracted
        .content_markdown
        .clone()
        .unwrap_or_else(|| extracted.content_text.clone());

    let mut asset_candidates = if let Some(base_url) = url {
        extracted
            .images
            .iter()
            .filter_map(|image| resolve_asset_candidate(base_url, &image.src))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if markdown.trim().is_empty() || extracted.extraction_quality < QUALITY_FALLBACK_THRESHOLD {
        let readability = Readability::new(
            &html,
            url.map(Url::as_str),
            Some(
                ReadabilityOptions::builder()
                    .remove_title_from_content(true)
                    .build(),
            ),
        )
        .context("readabilityrs 鍒濆鍖栧け璐?")?;
        if let Some(article) = readability.parse() {
            converter_chain.push("readabilityrs".to_string());
            title = title.or(article.title.clone());
            if let Some(article_html) = article.content.or(article.raw_content) {
                let converted = html_to_markdown(&article_html, None)
                    .context("readability HTML 杞?Markdown 澶辫触")?;
                converter_chain.push("html-to-markdown-rs".to_string());
                warnings.extend(
                    converted
                        .warnings
                        .into_iter()
                        .map(|warning| warning.message),
                );
                markdown = converted.content.unwrap_or(markdown);
                if let Some(base_url) = url {
                    asset_candidates.extend(collect_image_candidates(&article_html, base_url));
                }
                fidelity = "structured".to_string();
            } else if let Some(text_content) = article.text_content {
                markdown = text_content;
                fidelity = "plain".to_string();
            }
        }
    }

    if markdown.trim().is_empty() {
        let converted = html_to_markdown(&html, None).context("鏁翠綋 HTML 杞?Markdown 澶辫触")?;
        converter_chain.push("html-to-markdown-rs".to_string());
        warnings.extend(
            converted
                .warnings
                .into_iter()
                .map(|warning| warning.message),
        );
        markdown = converted.content.unwrap_or_default();
    }

    if markdown.trim().is_empty() {
        bail!("鏈兘浠?HTML 涓彁鍙栧埌鍙敤鍐呭");
    }

    let (assets, replacements, asset_warnings) = if let (Some(client), Some(_)) = (client, url) {
        download_remote_assets(client, &asset_candidates)?
    } else {
        (Vec::new(), BTreeMap::new(), Vec::new())
    };
    warnings.extend(asset_warnings);
    apply_replacements(&mut markdown, &replacements);

    let mut chain = if url.is_some() {
        vec!["reqwest".to_string()]
    } else {
        vec!["native".to_string()]
    };
    chain.extend(converter_chain);
    chain.push("comrak".to_string());

    Ok(normalize_bundle_markdown(PreparedBundle {
        title,
        source_url: url.map(ToString::to_string),
        source_path: None,
        source_type: "url".to_string(),
        platform: platform.to_string(),
        converter_chain: chain,
        fidelity,
        warnings,
        markdown: ensure_title_heading(markdown, extracted.metadata.title),
        mime: Some("text/html".to_string()),
        assets,
        source_files: Vec::new(),
    }))
}

fn convert_video_url(
    repo: &Repo,
    url: &Url,
    bundle_dir: &Utf8Path,
    request: &ConvertRequest<'_>,
) -> Result<PreparedBundle> {
    let source_dir = bundle_dir.join("source");
    let assets_dir = bundle_dir.join("assets");
    let resolved = sidecar::resolve_yt_dlp(repo).ok_or_else(|| {
        anyhow!(
            "failed to locate yt-dlp. install it with `llmwiki install-sidecar yt-dlp`, set `LLMWIKI_YT_DLP`, or add `yt-dlp` to PATH before converting bilibili or douyin video URLs"
        )
    })?;
    let mut command = Command::new(&resolved.path);
    command
        .arg("--no-playlist")
        .arg("--write-info-json")
        .arg("--write-thumbnail")
        .arg("--write-subs")
        .arg("--write-auto-subs")
        .arg("-P")
        .arg(source_dir.as_str())
        .arg("-o")
        .arg("item.%(ext)s");

    if !request.with_media {
        command.arg("--skip-download");
    }
    if let Some(user_agent) = request.user_agent {
        command.arg("--user-agent").arg(user_agent);
    }
    command.arg(url.as_str());

    let output = command.output().map_err(|error| {
        anyhow!(
            "failed to run yt-dlp at {}: {error}",
            resolved.path.display()
        )
    })?;
    if !output.status.success() {
        bail!(
            "yt-dlp 鎵ц澶辫触锛?{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let info_path = source_dir.join("item.info.json");
    if !info_path.exists() {
        bail!("yt-dlp 鏈敓鎴?item.info.json");
    }

    let info_text = fs::read_to_string(&info_path).context("璇诲彇 yt-dlp info.json 澶辫触")?;
    let info_json: Value =
        serde_json::from_str(&info_text).context("瑙ｆ瀽 yt-dlp info.json 澶辫触")?;

    let title = value_as_string(&info_json, &["title"]);
    let description = value_as_string(&info_json, &["description"]);
    let uploader = value_as_string(&info_json, &["uploader", "channel", "creator"]);
    let upload_date = value_as_string(&info_json, &["upload_date", "release_date"]);
    let duration = value_as_string(&info_json, &["duration_string"]).or_else(|| {
        info_json
            .get("duration")
            .and_then(|value| value.as_i64())
            .map(|seconds| seconds.to_string())
    });

    let mut assets = Vec::new();
    let mut subtitle_sections = Vec::new();
    for entry in fs::read_dir(&source_dir).context("璇诲彇 yt-dlp 杈撳嚭鐩綍澶辫触")?
    {
        let entry = entry.context("璇诲彇 yt-dlp 杈撳嚭鏉＄洰澶辫触")?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if filename.ends_with(".info.json") {
            continue;
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if is_video_asset_extension(&extension) || is_subtitle_extension(&extension) {
            let destination = assets_dir.join(filename);
            fs::rename(&path, &destination)
                .with_context(|| format!("failed to move yt-dlp output: {}", destination))?;
            if is_subtitle_extension(&extension) {
                let destination_utf8 = Utf8PathBuf::from_path_buf(destination.clone().into())
                    .map_err(|_| anyhow!("subtitle path is not valid UTF-8: {}", destination))?;
                let subtitle_text = read_subtitle_text(&destination_utf8)?;
                if !subtitle_text.is_empty() {
                    subtitle_sections.push(format!("### {}\n\n{}", filename, subtitle_text));
                }
            }
            assets.push(BundleFile {
                relative_path: format!("assets/{}", filename),
                bytes: fs::read(&destination).unwrap_or_default(),
            });
        }
    }

    let mut body = String::new();
    if let Some(title) = &title {
        body.push_str(&format!("# {}\n\n", title));
    }
    body.push_str("## Metadata\n\n");
    body.push_str(&format!("- URL: {}\n", url));
    if let Some(uploader) = &uploader {
        body.push_str(&format!("- Uploader: {}\n", uploader));
    }
    if let Some(upload_date) = &upload_date {
        body.push_str(&format!("- Upload date: {}\n", upload_date));
    }
    if let Some(duration) = &duration {
        body.push_str(&format!("- Duration: {}\n", duration));
    }
    body.push_str("\n## Description\n\n");
    body.push_str(description.as_deref().unwrap_or("_no description_"));
    if !subtitle_sections.is_empty() {
        body.push_str("\n\n## Subtitles\n\n");
        body.push_str(&subtitle_sections.join("\n\n"));
    }

    Ok(PreparedBundle {
        title,
        source_url: Some(url.to_string()),
        source_path: None,
        source_type: "video".to_string(),
        platform: detect_url_platform(url).to_string(),
        converter_chain: vec!["yt-dlp".to_string(), "comrak".to_string()],
        fidelity: "semi".to_string(),
        warnings: Vec::new(),
        markdown: normalize_markdown(&body),
        mime: Some("text/html".to_string()),
        assets,
        source_files: vec![BundleFile {
            relative_path: "source/item.info.json".to_string(),
            bytes: info_text.into_bytes(),
        }],
    })
}

fn write_bundle(bundle_dir: &Utf8Path, prepared: &PreparedBundle) -> Result<usize> {
    let captured_at = Utc::now().to_rfc3339();
    let frontmatter = NoteFrontmatter {
        source_url: prepared.source_url.clone(),
        source_path: prepared.source_path.clone(),
        source_type: prepared.source_type.clone(),
        platform: prepared.platform.clone(),
        converter_chain: prepared.converter_chain.clone(),
        captured_at: captured_at.clone(),
        fidelity: prepared.fidelity.clone(),
        warnings: prepared.warnings.clone(),
    };
    let note = render_note(&frontmatter, prepared.title.as_deref(), &prepared.markdown)?;

    fs::write(bundle_dir.join("note.md"), note)
        .with_context(|| format!("failed to write note.md: {}", bundle_dir.join("note.md")))?;

    let asset_metadata = prepared
        .assets
        .iter()
        .map(|asset| AssetMetadata {
            path: asset.relative_path.clone(),
            kind: asset_kind(&asset.relative_path).to_string(),
            source_url: None,
            bytes: asset.bytes.len(),
        })
        .collect::<Vec<_>>();
    let metadata = BundleMetadata {
        title: prepared.title.clone(),
        source_url: prepared.source_url.clone(),
        source_path: prepared.source_path.clone(),
        source_type: prepared.source_type.clone(),
        platform: prepared.platform.clone(),
        converter_chain: prepared.converter_chain.clone(),
        captured_at,
        fidelity: prepared.fidelity.clone(),
        warnings: prepared.warnings.clone(),
        mime: prepared.mime.clone(),
        assets: asset_metadata,
    };
    fs::write(
        bundle_dir.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).context("搴忓垪鍖?metadata.json 澶辫触")?,
    )
    .with_context(|| {
        format!(
            "failed to write metadata.json: {}",
            bundle_dir.join("metadata.json")
        )
    })?;

    for file in prepared.assets.iter().chain(prepared.source_files.iter()) {
        let target = bundle_dir.join(&file.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("鍒涘缓 bundle 瀛愮洰褰曞け璐ワ細{}", parent))?;
        }
        fs::write(&target, &file.bytes)
            .with_context(|| format!("failed to write bundle file: {}", target))?;
    }

    Ok(prepared.assets.len())
}

fn render_note(
    frontmatter: &NoteFrontmatter,
    title: Option<&str>,
    markdown: &str,
) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter).context("搴忓垪鍖?frontmatter 澶辫触")?;
    let mut note = String::from("---\n");
    note.push_str(&yaml);
    note.push_str("---\n\n");

    let body = if markdown.trim_start().starts_with("# ") || title.is_none() {
        markdown.trim().to_string()
    } else {
        format!("# {}\n\n{}", title.unwrap_or_default(), markdown.trim())
    };
    note.push_str(&body);
    note.push('\n');
    Ok(note)
}

fn normalize_bundle_markdown(mut bundle: PreparedBundle) -> PreparedBundle {
    bundle.markdown = normalize_markdown(&bundle.markdown);
    bundle
}

fn normalize_markdown(markdown: &str) -> String {
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    markdown_to_commonmark(markdown, &options)
        .trim()
        .to_string()
}

fn ensure_title_heading(markdown: String, title: Option<String>) -> String {
    if markdown.trim_start().starts_with("# ") {
        markdown
    } else if let Some(title) = title {
        format!("# {}\n\n{}", title, markdown.trim())
    } else {
        markdown
    }
}

fn detect_url_platform(url: &Url) -> &'static str {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if host.contains("mp.weixin.qq.com") {
        "weixin"
    } else if host.contains("zhihu.com") {
        "zhihu"
    } else if host.contains("bilibili.com") || host.contains("b23.tv") {
        "bilibili"
    } else if host.contains("douyin.com") || host.contains("iesdouyin.com") {
        "douyin"
    } else {
        "generic"
    }
}

fn select_first_text(document: &Html, selectors: &[&str]) -> Option<String> {
    for selector in selectors {
        let selector = Selector::parse(selector).ok()?;
        if let Some(node) = document.select(&selector).next() {
            let text = node.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

fn select_first_html(document: &Html, selectors: &[&str]) -> Option<String> {
    for selector in selectors {
        let selector = Selector::parse(selector).ok()?;
        if let Some(node) = document.select(&selector).next() {
            let html = node.inner_html();
            if !html.trim().is_empty() {
                return Some(html);
            }
        }
    }
    None
}

fn collect_image_candidates(html: &str, base_url: &Url) -> Vec<AssetCandidate> {
    let document = Html::parse_fragment(html);
    let selector = match Selector::parse("img") {
        Ok(selector) => selector,
        Err(_) => return Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut assets = Vec::new();

    for image in document.select(&selector) {
        let raw = image
            .value()
            .attr("data-src")
            .or_else(|| image.value().attr("src"))
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(raw) = raw {
            if let Some(candidate) = resolve_asset_candidate(base_url, raw) {
                if seen.insert(candidate.resolved.to_string()) {
                    assets.push(candidate);
                }
            }
        }
    }

    assets
}

fn resolve_asset_candidate(base_url: &Url, raw: &str) -> Option<AssetCandidate> {
    let resolved = base_url.join(raw).ok()?;
    Some(AssetCandidate {
        raw: raw.to_string(),
        resolved,
    })
}

fn download_remote_assets(
    client: &Client,
    assets: &[AssetCandidate],
) -> Result<(Vec<BundleFile>, BTreeMap<String, String>, Vec<String>)> {
    let mut files = Vec::new();
    let mut replacements = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut seen = BTreeSet::new();

    for asset in assets {
        if !seen.insert(asset.resolved.to_string()) {
            continue;
        }

        let response = match client.get(asset.resolved.clone()).send() {
            Ok(response) => match response.error_for_status() {
                Ok(response) => response,
                Err(error) => {
                    warnings.push(format!("failed to download asset: {error}"));
                    continue;
                }
            },
            Err(error) => {
                warnings.push(format!("failed to download asset: {error}"));
                continue;
            }
        };

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let response_url = response.url().clone();
        let bytes = match response.bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                warnings.push(format!("failed to read asset body: {error}"));
                continue;
            }
        };
        let extension = content_type
            .as_deref()
            .and_then(extension_from_mime)
            .or_else(|| {
                response_url
                    .path_segments()
                    .and_then(|mut segments| segments.next_back())
                    .and_then(infer_extension_from_name)
            })
            .unwrap_or("bin");
        let filename = format!("image-{:03}.{}", files.len() + 1, extension);
        let relative_path = format!("assets/{}", filename);

        replacements.insert(asset.raw.clone(), relative_path.clone());
        replacements.insert(asset.resolved.to_string(), relative_path.clone());
        files.push(BundleFile {
            relative_path,
            bytes: bytes.to_vec(),
        });
    }

    Ok((files, replacements, warnings))
}

fn apply_replacements(markdown: &mut String, replacements: &BTreeMap<String, String>) {
    for (from, to) in replacements {
        *markdown = markdown.replace(from, to);
    }
}

fn render_range_as_markdown(rows: &[&[Data]]) -> String {
    if rows.is_empty() {
        return "_empty sheet_".to_string();
    }

    let width = rows.iter().map(|row| row.len()).max().unwrap_or(0).max(1);
    let mut normalized = rows
        .iter()
        .map(|row| {
            let mut cells = row.iter().map(cell_to_string).collect::<Vec<_>>();
            while cells.len() < width {
                cells.push(String::new());
            }
            cells
        })
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        normalized.push(vec![String::new()]);
    }

    let header = normalized.remove(0);
    let mut out = String::new();
    out.push('|');
    out.push_str(
        &header
            .iter()
            .map(|cell| format!(" {}", escape_pipe(cell)))
            .collect::<Vec<_>>()
            .join(" |"),
    );
    out.push_str(" |\n|");
    out.push_str(&(0..width).map(|_| " --- ").collect::<Vec<_>>().join("|"));
    out.push_str("|\n");
    for row in normalized {
        out.push('|');
        out.push_str(
            &row.iter()
                .map(|cell| format!(" {}", escape_pipe(cell)))
                .collect::<Vec<_>>()
                .join(" |"),
        );
        out.push_str(" |\n");
    }
    out.trim_end().to_string()
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.clone(),
        Data::DurationIso(value) => value.clone(),
        Data::Error(value) => value.to_string(),
    }
}

fn escape_pipe(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

fn guess_plain_text(path: &Path) -> Result<bool> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read file: {}", path.display()))?;
    Ok(std::str::from_utf8(&bytes).is_ok())
}

fn mime_from_extension(extension: Option<&str>) -> Option<String> {
    match extension.unwrap_or_default().to_ascii_lowercase().as_str() {
        "pdf" => Some("application/pdf".to_string()),
        "docx" => Some(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
        ),
        "pptx" => Some(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
        ),
        "xlsx" => {
            Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string())
        }
        "xls" => Some("application/vnd.ms-excel".to_string()),
        "xlsm" => Some("application/vnd.ms-excel.sheet.macroEnabled.12".to_string()),
        "xlsb" => Some("application/vnd.ms-excel.sheet.binary.macroEnabled.12".to_string()),
        "ods" => Some("application/vnd.oasis.opendocument.spreadsheet".to_string()),
        "html" | "htm" => Some("text/html".to_string()),
        "md" | "markdown" => Some("text/markdown".to_string()),
        "txt" => Some("text/plain".to_string()),
        "json" => Some("application/json".to_string()),
        "xml" => Some("application/xml".to_string()),
        _ => None,
    }
}

fn extension_from_mime(mime: &str) -> Option<&'static str> {
    let normalized = mime
        .split(';')
        .next()
        .unwrap_or(mime)
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "image/svg+xml" => Some("svg"),
        "text/vtt" => Some("vtt"),
        "application/json" => Some("json"),
        "application/pdf" => Some("pdf"),
        _ => None,
    }
}

fn infer_extension_from_name(name: &str) -> Option<&str> {
    name.rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| !ext.is_empty())
}

fn extract_heading(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn asset_kind(path: &str) -> &'static str {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if is_subtitle_extension(&extension) {
        "subtitle"
    } else if is_video_asset_extension(&extension) {
        "image"
    } else {
        "binary"
    }
}

fn normalize_asset_filename(
    preferred: Option<&str>,
    fallback_stem: &str,
    mime: Option<&str>,
) -> String {
    let candidate = preferred
        .unwrap_or_default()
        .replace('\\', "/")
        .split('/')
        .filter(|segment| !segment.is_empty())
        .next_back()
        .unwrap_or_default()
        .trim()
        .to_string();

    if candidate.is_empty() || candidate == "." || candidate == ".." {
        return format!(
            "{}.{}",
            slugify(fallback_stem),
            mime.and_then(extension_from_mime).unwrap_or("bin")
        );
    }

    let has_extension = Path::new(&candidate)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| !ext.trim().is_empty());
    if has_extension {
        candidate
    } else {
        format!(
            "{}.{}",
            candidate,
            mime.and_then(extension_from_mime).unwrap_or("bin")
        )
    }
}

fn is_video_asset_extension(extension: &str) -> bool {
    matches!(extension, "jpg" | "jpeg" | "png" | "webp" | "gif")
}

fn is_subtitle_extension(extension: &str) -> bool {
    matches!(extension, "srt" | "vtt" | "lrc" | "ass" | "ssa" | "txt")
}

fn read_subtitle_text(path: &Utf8Path) -> Result<String> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read subtitle file: {}", path))?;
    let timecode = Regex::new(r"^\d{2}:\d{2}:\d{2}").expect("valid subtitle regex");
    let arrow = Regex::new(r"-->").expect("valid subtitle regex");
    let mut lines = Vec::new();
    let mut previous = String::new();

    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.eq_ignore_ascii_case("WEBVTT")
            || line.eq_ignore_ascii_case("NOTE")
            || line.chars().all(|ch| ch.is_ascii_digit())
            || timecode.is_match(line)
            || arrow.is_match(line)
        {
            continue;
        }
        if line != previous {
            lines.push(line.to_string());
            previous = line.to_string();
        }
    }

    Ok(lines.join("\n"))
}

fn value_as_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|candidate| match candidate {
            Value::String(value) if !value.is_empty() => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weixin_sidecar_command_reads_generated_markdown() {
        let script_dir = tempfile::tempdir().expect("script tempdir");
        let script_path = script_dir.path().join("wechat-article-to-markdown.cmd");
        fs::write(
            &script_path,
            "@echo off\r\nmkdir output\\sample >nul 2>nul\r\necho # Sidecar Title> output\\sample\\sample.md\r\necho.>> output\\sample\\sample.md\r\necho Sidecar body.>> output\\sample\\sample.md\r\n",
        )
        .expect("write sidecar script");

        let work_dir = tempfile::tempdir().expect("work tempdir");
        let url = Url::parse("https://mp.weixin.qq.com/s/test").expect("url");
        let bundle = run_weixin_sidecar_command(
            script_path.to_str().expect("script path"),
            &url,
            work_dir.path(),
        )
        .expect("run sidecar");

        assert_eq!(bundle.platform, "weixin");
        assert!(bundle.markdown.contains("Sidecar Title"));
        assert!(bundle.markdown.contains("Sidecar body."));
        assert!(
            bundle
                .source_files
                .iter()
                .any(|file| file.relative_path == "source/wechat-article-to-markdown.md")
        );
    }

    #[test]
    fn normalize_asset_filename_falls_back_when_name_is_empty() {
        let filename = normalize_asset_filename(Some(""), "slide-image", Some("image/png"));
        assert_eq!(filename, "slide-image.png");
    }
}
