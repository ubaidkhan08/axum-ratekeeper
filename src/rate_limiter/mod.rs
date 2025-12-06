pub mod gcra;
pub mod lua_scripts;
pub mod policy;
pub mod redis;
pub mod token_bucket;

use anyhow::Result;
use async_trait::async_trait;

pub use policy::{Algorithm, PolicyError, PolicyStore, RatePolicy};
pub use redis::RedisRateLimiter;

pub struct CheckInput<'a> {
    pub tenant_id: &'a str,
    pub identifier: &'a str,
    pub policy_key: &'a str,
    pub policy: &'a RatePolicy,
    pub cost: u32,
    pub now_ms: i64,
}

pub struct CheckOutput {
    pub allowed: bool,
    pub remaining: u32,
    pub retry_after_ms: i64,
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check(&self, input: CheckInput<'_>) -> Result<CheckOutput>;
}
