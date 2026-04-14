mod support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use predicates::str::contains;

#[test]
fn convert_markdown_file_writes_bundle_into_raw_inbox() {
    let repo = support::init_repo();
    support::write_file(
        &repo,
        "raw/inbox/source.md",
        "# Sample Title\n\nThis is a markdown source.\n",
    );

    let mut cmd = support::command_for(&repo);
    cmd.arg("convert").arg("raw/inbox/source.md");
    cmd.assert()
        .success()
        .stdout(contains("Bundle written to"))
        .stdout(contains("Platform: markdown"));

    let note = support::read_file(&repo, "raw/inbox/source/note.md");
    assert!(note.contains("platform: markdown"));
    assert!(note.contains("# Sample Title"));

    let metadata = support::read_file(&repo, "raw/inbox/source/metadata.json");
    assert!(metadata.contains("\"platform\": \"markdown\""));

    let copied = support::read_file(&repo, "raw/inbox/source/source/source.md");
    assert!(copied.contains("This is a markdown source."));
}

#[test]
fn convert_http_article_writes_markdown_bundle_and_assets() {
    let repo = support::init_repo();
    let server = spawn_test_server();

    let mut cmd = support::command_for(&repo);
    cmd.arg("convert").arg(server.article_url.as_str());
    cmd.assert().success().stdout(contains("Platform: generic"));

    server.join();

    let note = support::read_file(&repo, "raw/inbox/article/note.md");
    assert!(note.contains("Test Article"));
    assert!(note.contains("Hello from the article body."));

    let html = support::read_file(&repo, "raw/inbox/article/source/original.html");
    assert!(html.contains("<article>"));

    let asset_path = repo.path().join("raw/inbox/article/assets/image-001.png");
    assert!(asset_path.exists(), "expected downloaded image asset");
}

#[test]
fn convert_bilibili_url_requires_ytdlp_when_missing() {
    let repo = support::init_repo();
    let empty_path = tempfile::tempdir().expect("tempdir");

    let mut cmd = support::command_for(&repo);
    cmd.env("PATH", empty_path.path());
    cmd.arg("convert")
        .arg("https://www.bilibili.com/video/BV1xx411c7mD");
    cmd.assert().failure().stderr(contains("yt-dlp"));
}

#[test]
fn convert_bilibili_url_uses_repo_local_sidecar_when_path_is_empty() {
    let repo = support::init_repo();
    let empty_path = tempfile::tempdir().expect("tempdir");
    let sidecar_path = write_fake_repo_local_ytdlp(repo.path());

    let mut cmd = support::command_for(&repo);
    cmd.env("PATH", empty_path.path());
    cmd.arg("convert")
        .arg("https://www.bilibili.com/video/BV1xx411c7mD");
    cmd.assert()
        .success()
        .stdout(contains("Platform: bilibili"))
        .stdout(contains("Assets: 1"));

    assert!(
        sidecar_path.exists(),
        "expected repo-local sidecar to remain in place"
    );
    let note = support::read_file(&repo, "raw/inbox/bv1xx411c7md/note.md");
    assert!(note.contains("Repo Local Video"));
    let metadata = support::read_file(&repo, "raw/inbox/bv1xx411c7md/metadata.json");
    assert!(metadata.contains("\"platform\": \"bilibili\""));
}

#[test]
fn doctor_reports_convert_dependencies() {
    let repo = support::init_repo();
    let empty_path = tempfile::tempdir().expect("tempdir");

    let mut cmd = support::command_for(&repo);
    cmd.env("PATH", empty_path.path());
    cmd.arg("doctor");
    cmd.assert()
        .success()
        .stdout(contains("Convert doctor"))
        .stdout(contains("raw/inbox"))
        .stdout(contains("yt-dlp"))
        .stdout(contains("wechat-article-to-markdown"));
}

#[test]
fn doctor_reports_repo_local_ytdlp_path() {
    let repo = support::init_repo();
    let empty_path = tempfile::tempdir().expect("tempdir");
    write_fake_repo_local_ytdlp(repo.path());

    let mut cmd = support::command_for(&repo);
    cmd.env("PATH", empty_path.path());
    cmd.arg("doctor");
    cmd.assert()
        .success()
        .stdout(contains("repo-local sidecar"))
        .stdout(contains("yt-dlp"));
}

struct TestServer {
    article_url: String,
    handle: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn join(mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("server thread");
        }
    }
}

fn spawn_test_server() -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");
    let addr = listener.local_addr().expect("local addr");

    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).expect("set stream blocking");
                    let mut buffer = [0_u8; 4096];
                    let read = stream.read(&mut buffer).expect("read request");
                    let request = String::from_utf8_lossy(&buffer[..read]);
                    let path = request.split_whitespace().nth(1).unwrap_or("/");
                    match path {
                        "/article" => {
                            let body = r#"<!doctype html>
<html>
  <head>
    <title>Test Article</title>
  </head>
  <body>
    <article>
      <h1>Test Article</h1>
      <p>Hello from the article body.</p>
      <img src="/asset.png" alt="Pixel" />
    </article>
  </body>
</html>"#;
                            write_response(
                                &mut stream,
                                "200 OK",
                                "text/html; charset=utf-8",
                                body.as_bytes(),
                            );
                        }
                        "/asset.png" => {
                            write_response(&mut stream, "200 OK", "image/png", &tiny_png());
                        }
                        _ => write_response(
                            &mut stream,
                            "404 Not Found",
                            "text/plain; charset=utf-8",
                            b"missing",
                        ),
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("server accept failed: {error}"),
            }
        }
    });

    TestServer {
        article_url: format!("http://{addr}/article"),
        handle: Some(handle),
    }
}

fn write_response(stream: &mut std::net::TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).expect("write headers");
    stream.write_all(body).expect("write body");
    stream.flush().expect("flush");
}

fn tiny_png() -> Vec<u8> {
    vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 255, 255, 63, 0,
        5, 254, 2, 254, 167, 53, 129, 164, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]
}

fn write_fake_repo_local_ytdlp(repo_root: &Path) -> PathBuf {
    let sidecar_dir = repo_root.join("tools/yt-dlp").join(current_platform_dir());
    std::fs::create_dir_all(&sidecar_dir).expect("create sidecar dir");

    #[cfg(windows)]
    let path = {
        let script_path = sidecar_dir.join("yt-dlp.cmd");
        std::fs::write(
            &script_path,
            "@echo off\r\nsetlocal enabledelayedexpansion\r\nset \"out=\"\r\n:loop\r\nif \"%~1\"==\"\" goto done\r\nif \"%~1\"==\"-P\" (\r\n  set \"out=%~2\"\r\n  shift\r\n)\r\nshift\r\ngoto loop\r\n:done\r\nif not defined out exit /b 1\r\nmkdir \"%out%\" >nul 2>nul\r\n> \"%out%\\item.info.json\" echo {\"title\":\"Repo Local Video\",\"description\":\"Repo local description.\",\"uploader\":\"tester\"}\r\n> \"%out%\\item.jpg\" echo fake\r\nif \"%~1\"==\"--version\" echo test-sidecar-version\r\nexit /b 0\r\n",
        )
        .expect("write fake sidecar");
        script_path
    };

    #[cfg(not(windows))]
    let path = {
        let script_path = sidecar_dir.join("yt-dlp");
        std::fs::write(
            &script_path,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo test-sidecar-version\n  exit 0\nfi\nout=\"\"\nprev=\"\"\nfor arg in \"$@\"; do\n  if [ \"$prev\" = \"-P\" ]; then\n    out=\"$arg\"\n  fi\n  prev=\"$arg\"\ndone\nif [ -z \"$out\" ]; then\n  exit 1\nfi\nmkdir -p \"$out\"\ncat > \"$out/item.info.json\" <<'EOF'\n{\"title\":\"Repo Local Video\",\"description\":\"Repo local description.\",\"uploader\":\"tester\"}\nEOF\nprintf 'fake' > \"$out/item.jpg\"\n",
        )
        .expect("write fake sidecar");
        make_executable(&script_path);
        script_path
    };

    path
}

fn current_platform_dir() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", _) => "macos-universal",
        _ => "unsupported",
    }
}

#[cfg(not(windows))]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("set permissions");
}
