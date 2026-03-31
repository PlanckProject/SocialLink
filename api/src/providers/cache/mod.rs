mod in_process;
mod none;
mod redis;

use std::time::Duration;

use anyhow::Result;

pub use in_process::InProcessCache;
pub use none::NoCache;
pub use redis::RedisCache;

pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn delete_many(&self, keys: &[String]) -> Result<()>;
}

#[derive(Clone)]
pub enum CacheProvider {
    None(NoCache),
    InProcess(InProcessCache),
    Redis(Box<RedisCache>),
}

impl Cache for CacheProvider {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self {
            Self::None(cache) => cache.get(key).await,
            Self::InProcess(cache) => cache.get(key).await,
            Self::Redis(cache) => cache.get(key).await,
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        match self {
            Self::None(cache) => cache.set(key, value, ttl).await,
            Self::InProcess(cache) => cache.set(key, value, ttl).await,
            Self::Redis(cache) => cache.set(key, value, ttl).await,
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        match self {
            Self::None(cache) => cache.delete(key).await,
            Self::InProcess(cache) => cache.delete(key).await,
            Self::Redis(cache) => cache.delete(key).await,
        }
    }

    async fn delete_many(&self, keys: &[String]) -> Result<()> {
        match self {
            Self::None(cache) => cache.delete_many(keys).await,
            Self::InProcess(cache) => cache.delete_many(keys).await,
            Self::Redis(cache) => cache.delete_many(keys).await,
        }
    }
}
