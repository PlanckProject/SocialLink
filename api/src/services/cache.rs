use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::providers::cache::Cache;

#[derive(Clone)]
pub struct TypedCache<C> {
    provider: Arc<C>,
    ttl: Duration,
}

impl<C> TypedCache<C>
where
    C: Cache,
{
    pub fn new(provider: Arc<C>, ttl: Duration) -> Self {
        Self { provider, ttl }
    }

    pub async fn get<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let bytes = match self.provider.get(key).await {
            Ok(value) => value?,
            Err(error) => {
                tracing::warn!(
                    error = %format_args!("{error:#}"),
                    cache_namespace = namespace(key),
                    "cache read failed; using source of truth"
                );
                return None;
            }
        };

        match serde_json::from_slice(&bytes) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(
                    %error,
                    cache_namespace = namespace(key),
                    "cached value is invalid; evicting"
                );
                if let Err(delete_error) = self.provider.delete(key).await {
                    tracing::warn!(
                        error = %format_args!("{delete_error:#}"),
                        cache_namespace = namespace(key),
                        "failed to evict invalid cache value"
                    );
                }
                None
            }
        }
    }

    pub async fn set<T>(&self, key: &str, value: &T)
    where
        T: Serialize,
    {
        let bytes = match serde_json::to_vec(value) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(
                    %error,
                    cache_namespace = namespace(key),
                    "failed to serialize cache value"
                );
                return;
            }
        };

        if let Err(error) = self.provider.set(key, bytes, self.ttl).await {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                cache_namespace = namespace(key),
                "cache write failed"
            );
        }
    }

    pub async fn invalidate(&self, keys: &[String]) {
        if keys.is_empty() {
            return;
        }
        if let Err(error) = self.provider.delete_many(keys).await {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                cache_namespace = namespace(&keys[0]),
                key_count = keys.len(),
                "cache invalidation failed"
            );
        }
    }
}

fn namespace(key: &str) -> &str {
    key.split(':').next().unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use crate::providers::cache::InProcessCache;

    use super::*;

    #[tokio::test]
    async fn typed_cache_round_trips_and_invalidates_values() {
        let provider = Arc::new(InProcessCache::new(8).expect("cache"));
        let cache = TypedCache::new(provider, Duration::from_secs(60));
        let key = "person:id:test".to_string();

        cache.set(&key, &vec!["cached".to_string()]).await;
        assert_eq!(
            cache.get::<Vec<String>>(&key).await,
            Some(vec!["cached".to_string()])
        );

        cache.invalidate(std::slice::from_ref(&key)).await;
        assert_eq!(cache.get::<Vec<String>>(&key).await, None);
    }

    #[derive(Clone)]
    struct FailingCache;

    impl Cache for FailingCache {
        async fn get(&self, _key: &str) -> anyhow::Result<Option<Vec<u8>>> {
            bail!("cache unavailable")
        }

        async fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Duration) -> anyhow::Result<()> {
            bail!("cache unavailable")
        }

        async fn delete(&self, _key: &str) -> anyhow::Result<()> {
            bail!("cache unavailable")
        }

        async fn delete_many(&self, _keys: &[String]) -> anyhow::Result<()> {
            bail!("cache unavailable")
        }
    }

    #[tokio::test]
    async fn provider_failures_degrade_to_cache_misses() {
        let cache = TypedCache::new(Arc::new(FailingCache), Duration::from_secs(60));
        assert_eq!(cache.get::<String>("person:id:test").await, None);
        cache.set("person:id:test", &"value").await;
        cache.invalidate(&["person:id:test".to_string()]).await;
    }
}
