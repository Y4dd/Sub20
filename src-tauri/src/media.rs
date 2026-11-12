//! A tiny loopback file server for video previews on Linux.
//!
//! On Windows and macOS the window plays local files through Tauri's asset
//! protocol. WebKitGTK hands media to GStreamer, which only understands
//! ordinary URLs, so on Linux the same files are offered over HTTP instead:
//! bound to 127.0.0.1, on a random port, behind a random token, and only for
//! files the user has added to the bench.

use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type Files = Arc<Mutex<HashMap<String, PathBuf>>>;

pub struct MediaServer {
    port: u16,
    token: String,
    files: Files,
}

fn random_hex() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

impl MediaServer {
    pub async fn start() -> std::io::Result<MediaServer> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let token = random_hex();
        let files: Files = Arc::new(Mutex::new(HashMap::new()));

        let (shared_token, shared_files) = (token.clone(), files.clone());
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { continue };
                let (token, files) = (shared_token.clone(), shared_files.clone());
                tokio::spawn(async move {
                    let _ = serve(stream, &token, &files).await;
                });
            }
        });
        Ok(MediaServer { port, token, files })
    }

    /// Makes `path` playable and returns its URL.
    pub fn url_for(&self, path: &Path) -> String {
        let mut files = self.files.lock().expect("media lock");
        let id = match files.iter().find(|(_, known)| known.as_path() == path) {
            Some((id, _)) => id.clone(),
            None => {
                let id = random_hex();
                files.insert(id.clone(), path.to_path_buf());
                id
            }
        };
        format!("http://127.0.0.1:{}/{}/{}", self.port, self.token, id)
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("mp4" | "m4v") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        Some("ogv") => "video/ogg",
        Some("avi") => "video/x-msvideo",
        Some("ts" | "mts" | "m2ts") => "video/mp2t",
        _ => "application/octet-stream",
    }
}

/// Parses `bytes=a-b`, `bytes=a-` and `bytes=-n` against a file length.
/// Returns the inclusive byte range, or `None` when it cannot be satisfied.
fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    // Only the first range of a list is honored.
    let (start, end) = spec.split(',').next()?.trim().split_once('-')?;
    if len == 0 {
        return None;
    }
    let range = match (start.trim(), end.trim()) {
        ("", suffix) => {
            let n: u64 = suffix.parse().ok()?;
            (len.saturating_sub(n), len - 1)
        }
        (start, "") => (start.parse().ok()?, len - 1),
        (start, end) => (start.parse().ok()?, end.parse::<u64>().ok()?.min(len - 1)),
    };
    (range.0 <= range.1 && range.0 < len).then_some(range)
}

async fn reply(stream: &mut TcpStream, status: &str, extra: &str) -> std::io::Result<()> {
    let head = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n{extra}\r\n");
    stream.write_all(head.as_bytes()).await
}

async fn serve(mut stream: TcpStream, token: &str, files: &Files) -> std::io::Result<()> {
    // Read the request head.
    let mut head = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        head.extend_from_slice(&chunk[..n]);
        if head.len() > 16 * 1024 {
            return reply(&mut stream, "431 Request Header Fields Too Large", "").await;
        }
    }
    let head = String::from_utf8_lossy(&head);
    let mut lines = head.split("\r\n");
    let mut request = lines.next().unwrap_or("").split(' ');
    let (method, target) = (request.next().unwrap_or(""), request.next().unwrap_or(""));
    if method != "GET" && method != "HEAD" {
        return reply(&mut stream, "405 Method Not Allowed", "Allow: GET, HEAD\r\n").await;
    }

    // `/<token>/<id>`
    let mut parts = target.split('?').next().unwrap_or("").trim_start_matches('/').split('/');
    let (given_token, id) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let path = if given_token == token && parts.next().is_none() {
        files.lock().expect("media lock").get(id).cloned()
    } else {
        None
    };
    let Some(path) = path else {
        return reply(&mut stream, "404 Not Found", "").await;
    };

    let range_header = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("range"))
        .map(|(_, value)| value.to_string());

    let Ok(mut file) = tokio::fs::File::open(&path).await else {
        return reply(&mut stream, "404 Not Found", "").await;
    };
    let len = file.metadata().await?.len();
    let (status, start, end) = match range_header {
        Some(header) => match parse_range(&header, len) {
            Some((start, end)) => ("206 Partial Content", start, end),
            None => {
                return reply(&mut stream, "416 Range Not Satisfiable", &format!("Content-Range: bytes */{len}\r\n"))
                    .await;
            }
        },
        None => ("200 OK", 0, len.saturating_sub(1)),
    };
    let count = if len == 0 { 0 } else { end - start + 1 };

    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {count}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nConnection: close\r\n",
        content_type(&path)
    );
    if status.starts_with("206") {
        response.push_str(&format!("Content-Range: bytes {start}-{end}/{len}\r\n"));
    }
    response.push_str("\r\n");
    stream.write_all(response.as_bytes()).await?;
    if method == "HEAD" || count == 0 {
        return Ok(());
    }

    file.seek(SeekFrom::Start(start)).await?;
    // Stops by itself when the player hangs up mid-file, which it does on
    // every seek.
    tokio::io::copy(&mut file.take(count), &mut stream).await?;
    stream.shutdown().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range(" bytes=500-", 1000), Some((500, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=900-5000", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=0-0, 5-9", 1000), Some((0, 0)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=9-3", 1000), None);
        assert_eq!(parse_range("items=0-3", 1000), None);
        assert_eq!(parse_range("bytes=0-", 0), None);
    }

    #[tokio::test]
    async fn serves_registered_files_with_ranges_and_nothing_else() {
        let dir = std::env::temp_dir().join(format!("sub20-media-test-{}", random_hex()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("clip.mp4");
        std::fs::write(&file, b"0123456789").unwrap();
        let secret = dir.join("secret.txt");
        std::fs::write(&secret, b"private").unwrap();

        let server = MediaServer::start().await.unwrap();
        let url = server.url_for(&file);
        assert_eq!(url, server.url_for(&file), "the same file keeps its URL");
        let request_path = url.splitn(4, '/').nth(3).unwrap().to_string();

        let port = server.port;
        let host = format!("127.0.0.1:{port}");
        let fetch = |request: String| async move {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).await.unwrap();
            response
        };

        let whole = fetch(format!("GET /{request_path} HTTP/1.1\r\nHost: {host}\r\n\r\n")).await;
        assert!(whole.starts_with("HTTP/1.1 200 OK"), "{whole}");
        assert!(whole.contains("Content-Type: video/mp4") && whole.ends_with("0123456789"), "{whole}");

        let part = fetch(format!("GET /{request_path} HTTP/1.1\r\nHost: {host}\r\nrange: bytes=2-5\r\n\r\n")).await;
        assert!(part.starts_with("HTTP/1.1 206"), "{part}");
        assert!(part.contains("Content-Range: bytes 2-5/10") && part.ends_with("2345"), "{part}");

        let head = fetch(format!("HEAD /{request_path} HTTP/1.1\r\nHost: {host}\r\n\r\n")).await;
        assert!(head.contains("Content-Length: 10") && head.ends_with("\r\n\r\n"), "{head}");

        // Wrong token, unknown id, path tricks and other methods get nothing.
        for target in [
            format!("/wrong/{}", request_path.split('/').nth(1).unwrap()),
            format!("/{}/unknown", server.token),
            format!("/{}/../secret.txt", server.token),
            format!("/{}", secret.display()),
        ] {
            let response = fetch(format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n\r\n")).await;
            assert!(response.starts_with("HTTP/1.1 404"), "{target}: {response}");
        }
        let post = fetch(format!("POST /{request_path} HTTP/1.1\r\nHost: {host}\r\n\r\n")).await;
        assert!(post.starts_with("HTTP/1.1 405"), "{post}");

        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(&secret);
        let _ = std::fs::remove_dir(&dir);
    }
}
