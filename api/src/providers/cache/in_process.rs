use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Result, ensure};
use tokio::sync::RwLock;

use super::Cache;

#[derive(Clone)]
pub struct InProcessCache {
    entries: Arc<RwLock<HashMap<String, Entry>>>,
    max_entries: usize,
}

#[derive(Clone)]
struct Entry {
    value: Vec<u8>,
    expires_at: Instant,
}

impl InProcessCache {
    pub fn new(max_entries: usize) -> Result<Self> {
        ensure!(
            max_entries > 0,
            "in-process cache max_entries must be positive"
        );
        Ok(Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
        })
    }

    async fn remove_expired(&self, now: Instant) {
        self.entries
            .write()
            .await
            .retain(|_, entry| entry.expires_at > now);
    }
}

impl Cache for InProcessCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let now = Instant::now();
        {
            let entries = self.entries.read().await;
            if let Some(entry) = entries.get(key) {
                if entry.expires_at > now {
                    return Ok(Some(entry.value.clone()));
                }
            } else {
                return Ok(None);
            }
        }
        self.entries.write().await.remove(key);
        Ok(None)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        ensure!(!ttl.is_zero(), "cache TTL must be positive");
        let now = Instant::now();
        self.remove_expired(now).await;

        let mut entries = self.entries.write().await;
        let oldest_key = if entries.len() >= self.max_entries && !entries.contains_key(key) {
            entries
                .iter()
                .min_by_key(|(_, entry)| entry.expires_at)
                .map(|(key, _)| key.clone())
        } else {
            None
        };
        if let Some(oldest_key) = oldest_key {
            entries.remove(&oldest_key);
        }
        entries.insert(
            key.to_string(),
            Entry {
                value,
                expires_at: now + ttl,
            },
        );
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.entries.write().await.remove(key);
        Ok(())
    }

    async fn delete_many(&self, keys: &[String]) -> Result<()> {
        let mut entries = self.entries.write().await;
        for key in keys {
            entries.remove(key);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn expires_entries() {
        let cache = InProcessCache::new(4).expect("cache");
        cache
            .set("key", b"value".to_vec(), Duration::from_millis(10))
            .await
            .expect("set");
        assert_eq!(
            cache.get("key").await.expect("get"),
            Some(b"value".to_vec())
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(cache.get("key").await.expect("get"), None);
    }

    #[tokio::test]
    async fn remains_bounded() {
        let cache = InProcessCache::new(1).expect("cache");
        let ttl = Duration::from_secs(60);
        cache.set("one", vec![1], ttl).await.expect("set one");
        cache.set("two", vec![2], ttl).await.expect("set two");
        assert_eq!(cache.get("one").await.expect("get one"), None);
        assert_eq!(cache.get("two").await.expect("get two"), Some(vec![2]));
    }

    #[tokio::test]
    async fn miss_on_unknown_key() {
        let cache = InProcessCache::new(4).expect("cache");
        assert_eq!(cache.get("missing").await.expect("get"), None);
    }

    #[tokio::test]
    async fn hit_returns_stored_value() {
        let cache = InProcessCache::new(4).expect("cache");
        cache
            .set("key", b"value".to_vec(), Duration::from_secs(60))
            .await
            .expect("set");
        assert_eq!(
            cache.get("key").await.expect("get"),
            Some(b"value".to_vec())
        );
    }

    #[tokio::test]
    async fn overwrite_replaces_value_and_ttl() {
        let cache = InProcessCache::new(4).expect("cache");
        cache
            .set("key", b"first".to_vec(), Duration::from_secs(60))
            .await
            .expect("set first");
        cache
            .set("key", b"second".to_vec(), Duration::from_secs(60))
            .await
            .expect("set second");
        assert_eq!(
            cache.get("key").await.expect("get"),
            Some(b"second".to_vec())
        );
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let cache = InProcessCache::new(4).expect("cache");
        cache
            .set("key", b"value".to_vec(), Duration::from_secs(60))
            .await
            .expect("set");
        cache.delete("key").await.expect("delete");
        assert_eq!(cache.get("key").await.expect("get"), None);
    }

    #[tokio::test]
    async fn delete_many_removes_all_listed_keys() {
        let cache = InProcessCache::new(4).expect("cache");
        let ttl = Duration::from_secs(60);
        cache.set("one", vec![1], ttl).await.expect("set one");
        cache.set("two", vec![2], ttl).await.expect("set two");
        cache.set("three", vec![3], ttl).await.expect("set three");

        cache
            .delete_many(&["one".to_string(), "two".to_string()])
            .await
            .expect("delete_many");

        assert_eq!(cache.get("one").await.expect("get one"), None);
        assert_eq!(cache.get("two").await.expect("get two"), None);
        assert_eq!(cache.get("three").await.expect("get three"), Some(vec![3]));
    }

    #[tokio::test]
    async fn zero_ttl_is_rejected() {
        let cache = InProcessCache::new(4).expect("cache");
        let result = cache.set("key", vec![1], Duration::ZERO).await;
        assert!(result.is_err());
    }

    #[test]
    fn zero_capacity_is_rejected() {
        assert!(InProcessCache::new(0).is_err());
    }
}
