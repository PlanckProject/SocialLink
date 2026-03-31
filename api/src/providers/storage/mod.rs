mod local;

use std::path::PathBuf;

use anyhow::Result;
use bytes::Bytes;

pub use local::LocalStorage;

#[derive(Debug, Clone)]
pub struct StoredObject {
    pub bytes: Bytes,
    pub content_type: String,
}

pub trait ObjectStorage: Send + Sync {
    async fn put(&self, key: &str, bytes: Bytes, content_type: &str) -> Result<()>;
    async fn get(&self, key: &str) -> Result<Option<StoredObject>>;
    async fn delete(&self, key: &str) -> Result<()>;
}

#[derive(Clone)]
pub enum StorageProvider {
    Local(LocalStorage),
}

impl StorageProvider {
    pub async fn local(base_path: PathBuf) -> Result<Self> {
        Ok(Self::Local(LocalStorage::new(base_path).await?))
    }
}

impl ObjectStorage for StorageProvider {
    async fn put(&self, key: &str, bytes: Bytes, content_type: &str) -> Result<()> {
        match self {
            Self::Local(storage) => storage.put(key, bytes, content_type).await,
        }
    }

    async fn get(&self, key: &str) -> Result<Option<StoredObject>> {
        match self {
            Self::Local(storage) => storage.get(key).await,
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        match self {
            Self::Local(storage) => storage.delete(key).await,
        }
    }
}
