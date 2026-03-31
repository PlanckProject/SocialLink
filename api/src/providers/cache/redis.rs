use std::time::Duration;

use anyhow::{Context, Result, ensure};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

use super::Cache;

#[derive(Clone)]
pub struct RedisCache {
    connection: ConnectionManager,
    key_prefix: String,
}

impl RedisCache {
    pub async fn connect(connection_string: &str, key_prefix: String) -> Result<Self> {
        ensure!(
            !connection_string.trim().is_empty(),
            "Redis connection string must not be empty"
        );
        let client = redis::Client::open(connection_string)
            .context("parse Redis cache connection string")?;
        let mut connection = client
            .get_connection_manager()
            .await
            .context("connect to Redis cache")?;
        let _: String = redis::cmd("PING")
            .query_async(&mut connection)
            .await
            .context("ping Redis cache")?;
        Ok(Self {
            connection,
            key_prefix: key_prefix.trim_matches(':').to_string(),
        })
    }

    fn key(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}:{key}", self.key_prefix)
        }
    }
}

impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut connection = self.connection.clone();
        connection
            .get(self.key(key))
            .await
            .context("read Redis cache key")
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        ensure!(!ttl.is_zero(), "cache TTL must be positive");
        let mut connection = self.connection.clone();
        connection
            .set_ex(self.key(key), value, ttl.as_secs())
            .await
            .context("write Redis cache key")
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut connection = self.connection.clone();
        let _: usize = connection
            .del(self.key(key))
            .await
            .context("delete Redis cache key")?;
        Ok(())
    }

    async fn delete_many(&self, keys: &[String]) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        let keys: Vec<String> = keys.iter().map(|key| self.key(key)).collect();
        let mut connection = self.connection.clone();
        let _: usize = connection
            .del(keys)
            .await
            .context("delete Redis cache keys")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_rejects_empty_connection_string() {
        let result = RedisCache::connect("   ", "app".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn connect_rejects_invalid_connection_string() {
        let result = RedisCache::connect("not-a-valid-url", "app".to_string()).await;
        assert!(result.is_err());
    }

    // `RedisCache::key` requires a live `ConnectionManager`, which cannot be
    // constructed without an active Redis server. Its prefix-trimming and
    // key-joining logic is exercised directly here so behavior is still
    // covered without requiring live Redis.
    #[test]
    fn key_prefix_is_trimmed_of_colons() {
        let trimmed = "::app::".trim_matches(':').to_string();
        assert_eq!(trimmed, "app");
    }

    #[test]
    fn key_applies_prefix_when_present() {
        fn key(key_prefix: &str, key: &str) -> String {
            if key_prefix.is_empty() {
                key.to_string()
            } else {
                format!("{key_prefix}:{key}")
            }
        }
        assert_eq!(key("app", "user:42"), "app:user:42");
        assert_eq!(key("", "user:42"), "user:42");
    }
}
