use std::fmt::Debug;
use tokio::sync::RwLock;

use anyhow::{Context as _, Result};
use async_trait::async_trait;

use crate::{ingestion::IngestionNode, traits::NodeCache};

pub struct RedisNodeCache {
    client: redis::Client,
    connection_manager: RwLock<Option<redis::aio::ConnectionManager>>,
    key_prefix: String,
}

impl RedisNodeCache {
    pub fn try_from_url(url: &str, prefix: &str) -> Result<Self> {
        let client = redis::Client::open(url).context("Failed to open redis client")?;
        Ok(Self {
            client,
            connection_manager: RwLock::new(None),
            key_prefix: prefix.to_string(),
        })
    }

    // Connectionmanager is meant to be cloned
    async fn lazy_connect(&self) -> Option<redis::aio::ConnectionManager> {
        if self.connection_manager.read().await.is_none() {
            let result = self.client.get_connection_manager().await;
            if let Err(e) = result {
                tracing::error!("Failed to get connection manager: {}", e);
                return None;
            }
            let mut cm = self.connection_manager.write().await;
            *cm = result.ok();
        }

        self.connection_manager.read().await.clone()
    }

    fn key_for_node(&self, node: &IngestionNode) -> String {
        format!("{}:{}", self.key_prefix, node.calculate_hash())
    }

    #[allow(dead_code)]
    // Testing only, super inefficient
    async fn reset_cache(&self) {
        if let Some(mut cm) = self.lazy_connect().await {
            let keys: Vec<String> = redis::cmd("KEYS")
                .arg(format!("{}:*", self.key_prefix))
                .query_async(&mut cm)
                .await
                .expect("Could not get keys");

            for key in &keys {
                let _: usize = redis::cmd("DEL")
                    .arg(key)
                    .query_async(&mut cm)
                    .await
                    .expect("Failed to reset cache");
            }
        }
    }
}

// Redis CM does not implement debug
impl Debug for RedisNodeCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redis")
            .field("client", &self.client)
            .finish()
    }
}

#[async_trait]
impl NodeCache for RedisNodeCache {
    // false -> not cached, expect node to be processed
    // true -> cached, expect node to be skipped
    #[tracing::instrument(skip_all, name = "node_cache.redis.get", fields(hit))]
    async fn get(&self, node: &IngestionNode) -> bool {
        let cache_result = if let Some(mut cm) = self.lazy_connect().await {
            let result = redis::cmd("EXISTS")
                .arg(self.key_for_node(node))
                .query_async(&mut cm)
                .await;

            match result {
                Ok(1) => true,
                Ok(0) => false,
                Err(e) => {
                    tracing::error!("Failed to check node cache: {}", e);
                    false
                }
                _ => {
                    tracing::error!("Unexpected response from redis");
                    false
                }
            }
        } else {
            false
        };

        tracing::Span::current().record("hit", cache_result);

        cache_result
    }

    #[tracing::instrument(skip_all, name = "node_cache.redis.get")]
    async fn set(&self, node: &IngestionNode) {
        if let Some(mut cm) = self.lazy_connect().await {
            let result: Result<(), redis::RedisError> = redis::cmd("SET")
                .arg(self.key_for_node(node))
                .arg(1)
                .query_async(&mut cm)
                .await;

            if let Err(e) = result {
                tracing::error!("Failed to set node cache: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use testcontainers::runners::AsyncRunner;

    #[test_log::test(tokio::test)]
    async fn test_redis_cache() {
        let redis = testcontainers::GenericImage::new("redis", "7.2.4")
            .with_exposed_port(6379)
            .with_wait_for(testcontainers::core::WaitFor::message_on_stdout(
                "Ready to accept connections",
            ))
            .start()
            .await
            .expect("Redis started");

        let host = redis.get_host().await.unwrap();
        let port = redis.get_host_port_ipv4(6379).await.unwrap();
        let cache = RedisNodeCache::try_from_url(&format!("redis://{host}:{port}"), "test")
            .expect("Could not build redis client");
        cache.reset_cache().await;

        let node = IngestionNode {
            id: Some(1),
            path: "test".into(),
            chunk: "chunk".into(),
            vector: None,
            metadata: HashMap::new(),
        };

        let before_cache = cache.get(&node).await;
        assert!(!before_cache);

        cache.set(&node).await;
        let after_cache = cache.get(&node).await;
        assert!(after_cache);
    }
}
