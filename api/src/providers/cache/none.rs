use std::time::Duration;

use anyhow::Result;

use super::Cache;

#[derive(Debug, Clone, Copy, Default)]
pub struct NoCache;

impl Cache for NoCache {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Duration) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, _key: &str) -> Result<()> {
        Ok(())
    }

    async fn delete_many(&self, _keys: &[String]) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn always_misses_and_no_ops() {
        let cache = NoCache;
        cache
            .set("key", b"value".to_vec(), Duration::from_secs(60))
            .await
            .expect("set");
        assert_eq!(cache.get("key").await.expect("get"), None);
        cache.delete("key").await.expect("delete");
        cache
            .delete_many(&["a".to_string(), "b".to_string()])
            .await
            .expect("delete_many");
    }
}
