//! Bounded object previews and streaming, no-overwrite downloads.
use super::S3Client;
use anyhow::{anyhow, bail, ensure, Result};
use aws_smithy_types::byte_stream::ByteStream;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::io::AsyncWriteExt;

pub const PREVIEW_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Json,
    Yaml,
    Plain,
}

#[derive(Debug)]
pub struct TextPreview {
    pub text: String,
    pub format: TextFormat,
    pub truncated: bool,
}

impl S3Client {
    pub async fn preview_object(
        &self,
        bucket: &str,
        region: &str,
        key: &str,
    ) -> Result<TextPreview> {
        let client = self.config.s3_client_with_region(region).await;
        let response = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .range(format!("bytes=0-{PREVIEW_LIMIT}"))
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error)
                if error
                    .raw_response()
                    .is_some_and(|response| response.status().as_u16() == 416) =>
            {
                let metadata = client.head_object().bucket(bucket).key(key).send().await?;
                ensure!(
                    metadata.content_length() == Some(0),
                    "Object range is unavailable; try downloading it"
                );
                return decode_preview(
                    key,
                    metadata.content_type().unwrap_or_default(),
                    Vec::new(),
                );
            }
            Err(error) => return Err(error.into()),
        };
        let content_type = response.content_type().unwrap_or_default().to_string();
        decode_preview(key, &content_type, read_prefix(response.body).await?)
    }

    pub async fn download_object(
        &self,
        bucket: &str,
        region: &str,
        key: &str,
        directory: &Path,
    ) -> Result<PathBuf> {
        let destination = directory.join(download_name(key)?);
        ensure!(
            !destination.try_exists()?,
            "File already exists: {}",
            destination.display()
        );
        let client = self.config.s3_client_with_region(region).await;
        let response = client.get_object().bucket(bucket).key(key).send().await?;
        save_stream(response.body, response.content_length, destination).await
    }
}

async fn read_prefix(mut body: ByteStream) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = body.try_next().await? {
        let remaining = (PREVIEW_LIMIT + 1).saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if bytes.len() > PREVIEW_LIMIT {
            break;
        }
    }
    Ok(bytes)
}

fn decode_preview(key: &str, content_type: &str, mut bytes: Vec<u8>) -> Result<TextPreview> {
    let truncated = bytes.len() > PREVIEW_LIMIT;
    bytes.truncate(PREVIEW_LIMIT);
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(error) if truncated && error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()])?
        }
        Err(_) => bail!("Preview supports UTF-8 text only; use Ctrl+D to download this object"),
    };
    ensure!(
        !text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')),
        "Binary or control-character content cannot be previewed; use Ctrl+D to download"
    );
    let text = text.trim_start_matches('\u{feff}');
    let extension = key
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    ensure!(
        mime.is_empty()
            || mime.starts_with("text/")
            || mime.ends_with("+json")
            || mime.ends_with("+xml")
            || matches!(
                mime.as_str(),
                "application/octet-stream"
                    | "application/json"
                    | "application/yaml"
                    | "application/x-yaml"
                    | "application/xml"
                    | "application/javascript"
                    | "application/toml"
                    | "application/x-sh"
            ),
        "Preview supports text formats only ({mime}); use Ctrl+D to download"
    );
    ensure!(
        !matches!(
            extension.as_str(),
            "pdf"
                | "zip"
                | "gz"
                | "tgz"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "parquet"
                | "avro"
                | "exe"
                | "wasm"
                | "mp4"
                | "mp3"
        ),
        "Preview supports text formats only; use Ctrl+D to download"
    );
    let format = if extension == "json" || mime == "application/json" || mime.ends_with("+json") {
        TextFormat::Json
    } else if matches!(extension.as_str(), "yaml" | "yml")
        || matches!(
            mime.as_str(),
            "application/yaml" | "application/x-yaml" | "text/yaml"
        )
    {
        TextFormat::Yaml
    } else {
        TextFormat::Plain
    };
    // Pretty-print only complete, valid JSON; preserve truncated/invalid text for inspection.
    let formatted = if format == TextFormat::Json && !truncated {
        serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok())
            .filter(|pretty| pretty.len() <= PREVIEW_LIMIT)
    } else {
        None
    };
    Ok(TextPreview {
        text: formatted.unwrap_or_else(|| text.into()),
        format,
        truncated,
    })
}

fn download_name(key: &str) -> Result<String> {
    let name = key.rsplit(['/', '\\']).next().unwrap_or_default();
    ensure!(
        !name.is_empty() && name != "." && name != "..",
        "Object has no safe filename"
    );
    // Do not interpret S3 keys as local paths, or allow terminal controls in notices.
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    Ok(name)
}

struct PartialFile(PathBuf);
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
static DOWNLOAD_ID: AtomicU64 = AtomicU64::new(0);

async fn save_stream(
    mut body: ByteStream,
    expected: Option<i64>,
    destination: PathBuf,
) -> Result<PathBuf> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("Download directory is missing"))?;
    let mut opened = None;
    for _ in 0..100 {
        let path = parent.join(format!(
            ".rusticity-download-{}-{}.part",
            std::process::id(),
            DOWNLOAD_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        // Create and register cleanup without an await/cancellation gap.
        match options.open(&path) {
            Ok(file) => {
                opened = Some((PartialFile(path), tokio::fs::File::from_std(file)));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    let (partial, mut file) =
        opened.ok_or_else(|| anyhow!("Could not create a unique partial download"))?;
    let mut written = 0u64;
    while let Some(chunk) = body.try_next().await? {
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
    }
    if let Some(expected) = expected {
        ensure!(
            expected >= 0 && written == expected as u64,
            "Incomplete download: received {written} of {expected} bytes"
        );
    }
    file.flush().await?;
    file.sync_all().await?;
    drop(file);
    // Linking in the same directory atomically publishes the completed file and
    // fails if *anything* already occupies the destination (including a symlink).
    // This short metadata operation is synchronous so task cancellation cannot
    // race publication after the UI has reported cancellation.
    std::fs::hard_link(&partial.0, &destination).map_err(|error| {
        anyhow!(
            "Cannot save {} (existing files are never overwritten): {error}",
            destination.display()
        )
    })?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rusticity-download-test-{}-{}",
            std::process::id(),
            DOWNLOAD_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn text_formats_binary_and_utf8_boundaries() {
        let json = decode_preview("dir/object.json", "", br#"{"hello":true}"#.to_vec()).unwrap();
        assert_eq!(json.format, TextFormat::Json);
        assert!(json.text.contains("\n  \"hello\": true"));
        assert_eq!(
            decode_preview("x.YML", "", b"enabled: true".to_vec())
                .unwrap()
                .format,
            TextFormat::Yaml
        );
        assert_eq!(
            decode_preview("x.txt", "text/plain", b"hello".to_vec())
                .unwrap()
                .format,
            TextFormat::Plain
        );
        assert!(decode_preview("x.json", "", vec![0xff]).is_err());
        assert!(decode_preview("x.txt", "", b"bad\x1b[2J".to_vec()).is_err());
        assert!(decode_preview("x.bin", "", b"abc\0def".to_vec()).is_err());
        assert_eq!(decode_preview("empty.txt", "", vec![]).unwrap().text, "");
        assert!(decode_preview("x.txt", "application/pdf", b"%PDF-ascii".to_vec()).is_err());
        assert!(
            decode_preview("x.pdf", "application/octet-stream", b"%PDF-ascii".to_vec()).is_err()
        );
        let mut bytes = vec![b'a'; PREVIEW_LIMIT - 1];
        bytes.extend_from_slice("界".as_bytes());
        let preview = decode_preview("x.txt", "", bytes).unwrap();
        assert!(preview.truncated);
        assert_eq!(preview.text.len(), PREVIEW_LIMIT - 1);
    }
    #[test]
    fn object_keys_cannot_escape_download_directory() {
        assert_eq!(
            download_name("../../some/object.json").unwrap(),
            "object.json"
        );
        assert_eq!(download_name("a\\b.txt").unwrap(), "b.txt");
        for key in ["", "folder/", ".", "..", "folder/.."] {
            assert!(download_name(key).is_err());
        }
        assert_eq!(download_name("bad\nname.txt").unwrap(), "bad_name.txt");
    }
    #[tokio::test]
    async fn download_publishes_exact_bytes_and_never_overwrites() {
        let directory = directory();
        let target = directory.join("object.txt");
        assert_eq!(
            save_stream(ByteStream::from_static(b"hello"), Some(5), target.clone())
                .await
                .unwrap(),
            target
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"hello");
        assert!(
            save_stream(ByteStream::from_static(b"different"), None, target.clone())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"hello");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[tokio::test]
    async fn incomplete_download_removes_partial_and_does_not_publish() {
        let directory = directory();
        let target = directory.join("object.txt");
        assert!(save_stream(
            ByteStream::from_static(b"partial"),
            Some(100),
            target.clone()
        )
        .await
        .is_err());
        assert!(!target.exists());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[tokio::test]
    async fn cancelled_download_removes_its_partial_file() {
        let directory = directory();
        let target = directory.join("large.bin");
        let destination = target.clone();
        let task = tokio::spawn(async move {
            save_stream(
                ByteStream::from(vec![42; 8 * 1024 * 1024]),
                None,
                destination,
            )
            .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while std::fs::read_dir(&directory).unwrap().count() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!target.exists());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn downloads_do_not_replace_symlinks_and_publish_private_permissions() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let directory = directory();
        let target = directory.join("object.bin");
        let unrelated = directory.join("unrelated.bin");
        std::fs::write(&unrelated, b"keep").unwrap();
        symlink(&unrelated, &target).unwrap();
        assert!(
            save_stream(ByteStream::from_static(b"new"), None, target.clone())
                .await
                .is_err()
        );
        assert!(std::fs::symlink_metadata(&target).unwrap().is_symlink());
        assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
        let private = directory.join("private.bin");
        save_stream(ByteStream::from_static(b"new"), None, private.clone())
            .await
            .unwrap();
        assert_eq!(
            std::fs::metadata(private).unwrap().permissions().mode() & 0o777,
            0o600
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn preview_reader_stops_at_bounded_prefix() {
        let bytes = read_prefix(ByteStream::from(vec![b'x'; PREVIEW_LIMIT * 2]))
            .await
            .unwrap();
        assert_eq!(bytes.len(), PREVIEW_LIMIT + 1);
    }
}
