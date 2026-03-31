use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use bytes::Bytes;

use super::{ObjectStorage, StoredObject};

#[derive(Clone)]
pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub async fn new(base_path: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&base_path)
            .await
            .with_context(|| {
                format!(
                    "create local object storage directory {}",
                    base_path.display()
                )
            })?;
        Ok(Self { base_path })
    }

    fn resolve(&self, key: &str) -> Result<PathBuf> {
        let key_path = Path::new(key);
        if key.is_empty()
            || key.contains('\\')
            || key
                .split('/')
                .next()
                .is_some_and(|segment| segment.contains(':'))
            || key_path.is_absolute()
            || key_path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            bail!("invalid object storage key");
        }
        Ok(self.base_path.join(key_path))
    }
}

impl ObjectStorage for LocalStorage {
    async fn put(&self, key: &str, bytes: Bytes, content_type: &str) -> Result<()> {
        let path = self.resolve(key)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create object parent directory {}", parent.display()))?;
        }
        tokio::fs::write(&path, bytes)
            .await
            .with_context(|| format!("write object {}", path.display()))?;

        let ctype_path = content_type_path(&path);
        tokio::fs::write(&ctype_path, content_type.as_bytes())
            .await
            .with_context(|| format!("write object content type {}", ctype_path.display()))
    }

    async fn get(&self, key: &str) -> Result<Option<StoredObject>> {
        let path = self.resolve(key)?;
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error).with_context(|| format!("read object {}", path.display()));
            }
        };
        let content_type = match tokio::fs::read_to_string(content_type_path(&path)).await {
            Ok(content_type) if !content_type.trim().is_empty() => content_type.trim().to_string(),
            _ => content_type_for(&path).to_string(),
        };
        Ok(Some(StoredObject {
            bytes: Bytes::from(bytes),
            content_type,
        }))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.resolve(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("delete object {}", path.display()));
            }
        }

        match tokio::fs::remove_file(content_type_path(&path)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| {
                format!(
                    "delete object content type {}",
                    content_type_path(&path).display()
                )
            }),
        }
    }
}

/// Sidecar file path that stores the exact content type supplied on `put`, so
/// `get` can report it back robustly instead of only guessing from extension.
fn content_type_path(path: &Path) -> PathBuf {
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(".ctype");
    path.with_file_name(file_name)
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "social-link-storage-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ))
    }

    #[tokio::test]
    async fn round_trips_objects() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        storage
            .put(
                "avatars/example.png",
                Bytes::from_static(b"image"),
                "image/png",
            )
            .await
            .expect("put");

        let stored = storage
            .get("avatars/example.png")
            .await
            .expect("get")
            .expect("object");
        assert_eq!(stored.bytes, Bytes::from_static(b"image"));
        assert_eq!(stored.content_type, "image/png");

        storage.delete("avatars/example.png").await.expect("delete");
        assert!(
            storage
                .get("avatars/example.png")
                .await
                .expect("get")
                .is_none()
        );
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn round_trips_deeply_nested_keys() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        storage
            .put(
                "users/1/avatars/2024/example.webp",
                Bytes::from_static(b"nested"),
                "image/webp",
            )
            .await
            .expect("put");

        let stored = storage
            .get("users/1/avatars/2024/example.webp")
            .await
            .expect("get")
            .expect("object");
        assert_eq!(stored.bytes, Bytes::from_static(b"nested"));
        assert_eq!(stored.content_type, "image/webp");
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        assert!(
            storage
                .put("../escape", Bytes::new(), "application/octet-stream")
                .await
                .is_err()
        );
        assert!(
            storage
                .put(
                    "nested/../../escape",
                    Bytes::new(),
                    "application/octet-stream"
                )
                .await
                .is_err()
        );
        assert!(storage.get("../escape").await.is_err());
        assert!(storage.delete("../escape").await.is_err());
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn rejects_absolute_and_empty_keys() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        assert!(
            storage
                .put("", Bytes::new(), "application/octet-stream")
                .await
                .is_err()
        );
        assert!(
            storage
                .put("/etc/passwd", Bytes::new(), "application/octet-stream")
                .await
                .is_err()
        );
        assert!(
            storage
                .put(
                    r"C:\Windows\system.ini",
                    Bytes::new(),
                    "application/octet-stream"
                )
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn guesses_content_type_when_not_recorded() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        // Write directly to disk without going through `put`, so no sidecar
        // content-type file exists; `get` must fall back to extension sniffing.
        let file_path = base.join("direct.jpg");
        tokio::fs::write(&file_path, b"raw")
            .await
            .expect("write raw file");

        let stored = storage
            .get("direct.jpg")
            .await
            .expect("get")
            .expect("object");
        assert_eq!(stored.content_type, "image/jpeg");
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn falls_back_to_octet_stream_for_unknown_extension() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        let file_path = base.join("data.bin");
        tokio::fs::write(&file_path, b"raw")
            .await
            .expect("write raw file");

        let stored = storage.get("data.bin").await.expect("get").expect("object");
        assert_eq!(stored.content_type, "application/octet-stream");
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn get_returns_none_for_missing_key() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        assert!(storage.get("missing/key.png").await.expect("get").is_none());
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        storage
            .put("file.png", Bytes::from_static(b"data"), "image/png")
            .await
            .expect("put");

        storage.delete("file.png").await.expect("first delete");
        storage
            .delete("file.png")
            .await
            .expect("second delete on missing file");
        assert!(storage.get("file.png").await.expect("get").is_none());
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }

    #[tokio::test]
    async fn overwrites_content_type_on_put() {
        let base = temp_path();
        let storage = LocalStorage::new(base.clone()).await.expect("storage");
        storage
            .put("thing.png", Bytes::from_static(b"a"), "image/png")
            .await
            .expect("put png");
        storage
            .put("thing.png", Bytes::from_static(b"b"), "application/pdf")
            .await
            .expect("put pdf");

        let stored = storage
            .get("thing.png")
            .await
            .expect("get")
            .expect("object");
        assert_eq!(stored.bytes, Bytes::from_static(b"b"));
        assert_eq!(stored.content_type, "application/pdf");
        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }
}
