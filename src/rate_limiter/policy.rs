use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use moka::future::Cache;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Algorithm {
    TokenBucket,
    Gcra,
}

impl Default for Algorithm {
    fn default() -> Self {
        Algorithm::TokenBucket
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatePolicy {
    pub capacity: u32,
    pub refill_tokens_per_sec: f64,
    #[serde(default)]
    pub algorithm: Algorithm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPolicy {
    pub tenant_id: String,
    pub api_key: Option<String>,
    pub policies: HashMap<String, RatePolicy>,
}

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("policy not found")]
    NotFound,
}

pub struct PolicyStore {
    policy_cache: Cache<String, RatePolicy>,
    api_key_cache: Cache<String, Option<String>>,
    redis: ConnectionManager,
    _ttl: Duration,
}

impl PolicyStore {
    pub fn new(redis: ConnectionManager, ttl: Duration) -> Self {
        let policy_cache = Cache::builder().time_to_live(ttl).build();
        let api_key_cache = Cache::builder().time_to_live(ttl).build();
        Self {
            policy_cache,
            api_key_cache,
            redis,
            _ttl: ttl,
        }
    }

    pub async fn get_policy(&self, tenant_id: &str, policy_key: &str) -> Result<RatePolicy> {
        let cache_key = format!("{tenant_id}:{policy_key}");
        if let Some(policy) = self.policy_cache.get(&cache_key).await {
            return Ok(policy);
        }

        let policy = self.fetch_policy(tenant_id, policy_key).await?;
        self.policy_cache.insert(cache_key, policy.clone()).await;
        Ok(policy)
    }

    pub async fn get_api_key(&self, tenant_id: &str) -> Result<Option<String>> {
        if let Some(key) = self.api_key_cache.get(tenant_id).await {
            return Ok(key);
        }

        let value = self.fetch_api_key(tenant_id).await?;
        self.api_key_cache
            .insert(tenant_id.to_string(), value.clone())
            .await;
        Ok(value)
    }

    pub async fn set_policy(
        &self,
        tenant_id: &str,
        policy_key: &str,
        policy: RatePolicy,
    ) -> Result<()> {
        let redis_key = format!("rl:tenant:{tenant_id}");
        let field = format!("policy:{policy_key}");

        let json = serde_json::to_string(&policy)?;
        let mut conn = self.redis.clone();
        let _: () = redis::cmd("HSET")
            .arg(redis_key)
            .arg(field)
            .arg(json)
            .query_async(&mut conn)
            .await?;

        let cache_key = format!("{tenant_id}:{policy_key}");
        self.policy_cache.invalidate(&cache_key).await;
        Ok(())
    }

    pub async fn set_api_key(&self, tenant_id: &str, api_key: Option<String>) -> Result<()> {
        let redis_key = format!("rl:tenant:{tenant_id}");
        let mut conn = self.redis.clone();

        match api_key {
            Some(ref key) => {
                let _: () = redis::cmd("HSET")
                    .arg(&redis_key)
                    .arg("api_key")
                    .arg(key)
                    .query_async(&mut conn)
                    .await?;
            }
            None => {
                let _: () = redis::cmd("HDEL")
                    .arg(&redis_key)
                    .arg("api_key")
                    .query_async(&mut conn)
                    .await?;
            }
        }

        self.api_key_cache.invalidate(tenant_id).await;
        Ok(())
    }

    async fn fetch_policy(&self, tenant_id: &str, policy_key: &str) -> Result<RatePolicy> {
        let redis_key = format!("rl:tenant:{tenant_id}");
        let field = format!("policy:{policy_key}");

        let mut conn = self.redis.clone();
        let value: Option<String> = redis::cmd("HGET")
            .arg(redis_key)
            .arg(field)
            .query_async(&mut conn)
            .await?;

        match value {
            Some(json) => {
                let policy: RatePolicy = serde_json::from_str(&json)?;
                Ok(policy)
            }
            None => Err(PolicyError::NotFound.into()),
        }
    }

    async fn fetch_api_key(&self, tenant_id: &str) -> Result<Option<String>> {
        let redis_key = format!("rl:tenant:{tenant_id}");
        let mut conn = self.redis.clone();

        let value: Option<String> = redis::cmd("HGET")
            .arg(redis_key)
            .arg("api_key")
            .query_async(&mut conn)
            .await?;

        Ok(value)
    }
}
