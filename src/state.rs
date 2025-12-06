use std::sync::Arc;

use anyhow::Result;

use crate::{
    config::Config,
    metrics::Metrics,
    rate_limiter::{PolicyStore, RedisRateLimiter},
    redis_pool::create_redis_pool,
};

pub struct AppState {
    pub config: Config,
    pub policy_store: PolicyStore,
    pub rate_limiter: RedisRateLimiter,
    pub metrics: Metrics,
}

impl AppState {
    pub async fn initialize(config: Config) -> Result<Arc<Self>> {
        let redis = create_redis_pool(&config.redis_url).await?;
        let policy_store = PolicyStore::new(redis.clone(), config.policy_cache_ttl);
        let rate_limiter = RedisRateLimiter::new(
            redis.clone(),
            config.redis_max_retries,
            config.redis_backoff_ms,
        )?;

        Ok(Arc::new(Self {
            config,
            policy_store,
            rate_limiter,
            metrics: Metrics::new(),
        }))
    }
}
