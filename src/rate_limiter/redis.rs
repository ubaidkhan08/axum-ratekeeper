use anyhow::{anyhow, Result};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, RedisError};
use tokio::time::{sleep, Duration};

use super::{
    gcra::{compute_gcra_ttl, GCRA_LUA},
    policy::Algorithm,
    token_bucket::{compute_ttl, token_bucket_script},
    CheckInput, CheckOutput, RateLimiter,
};

fn is_retryable(err: &RedisError) -> bool {
    use redis::ErrorKind;
    matches!(
        err.kind(),
        ErrorKind::IoError
            | ErrorKind::BusyLoadingError
            | ErrorKind::TryAgain
            | ErrorKind::ResponseError
    )
}

pub struct RedisRateLimiter {
    pub redis: ConnectionManager,
    token_script: redis::Script,
    gcra_script: redis::Script,
    max_retries: u32,
    backoff_ms: u64,
}

impl RedisRateLimiter {
    pub fn new(redis: ConnectionManager, max_retries: u32, backoff_ms: u64) -> Result<Self> {
        let token_script = token_bucket_script();
        let gcra_script = redis::Script::new(GCRA_LUA);
        Ok(Self {
            redis,
            token_script,
            gcra_script,
            max_retries,
            backoff_ms,
        })
    }

    async fn invoke_token_bucket(&self, input: &CheckInput<'_>) -> Result<CheckOutput> {
        let key = format!(
            "rl:bucket:{}:{}:{}",
            input.tenant_id, input.policy_key, input.identifier
        );
        let ttl = compute_ttl(input.policy);
        let mut conn = self.redis.clone();
        let mut invocation = self.token_script.prepare_invoke();

        invocation.key(key);
        invocation.arg(input.policy.capacity);
        invocation.arg(input.policy.refill_tokens_per_sec);
        invocation.arg(input.now_ms);
        invocation.arg(input.cost);
        invocation.arg(ttl);

        let (allowed, remaining, retry_after): (i32, i32, i32) =
            invocation.invoke_async(&mut conn).await?;

        Ok(CheckOutput {
            allowed: allowed == 1,
            remaining: remaining as u32,
            retry_after_ms: retry_after as i64,
        })
    }

    async fn invoke_gcra(&self, input: &CheckInput<'_>) -> Result<CheckOutput> {
        let key = format!(
            "rl:gcra:{}:{}:{}",
            input.tenant_id, input.policy_key, input.identifier
        );
        let ttl = compute_gcra_ttl(input.policy);
        let mut conn = self.redis.clone();
        let mut invocation = self.gcra_script.prepare_invoke();

        invocation.key(key);
        invocation.arg(input.policy.capacity);
        invocation.arg(input.policy.refill_tokens_per_sec);
        invocation.arg(input.now_ms);
        invocation.arg(input.cost);
        invocation.arg(ttl);

        let (allowed, remaining, retry_after): (i32, i32, i32) =
            invocation.invoke_async(&mut conn).await?;

        Ok(CheckOutput {
            allowed: allowed == 1,
            remaining: remaining as u32,
            retry_after_ms: retry_after as i64,
        })
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, input: CheckInput<'_>) -> Result<CheckOutput> {
        let mut last_err: Option<String> = None;
        for attempt in 0..=self.max_retries {
            let result = match input.policy.algorithm {
                Algorithm::TokenBucket => self.invoke_token_bucket(&input).await,
                Algorithm::Gcra => self.invoke_gcra(&input).await,
            };

            match result {
                Ok(res) => return Ok(res),
                Err(e) => {
                    if let Some(redis_err) = e.downcast_ref::<RedisError>() {
                        if attempt < self.max_retries && is_retryable(redis_err) {
                            last_err = Some(redis_err.to_string());
                            let backoff =
                                Duration::from_millis(self.backoff_ms * (attempt + 1) as u64);
                            sleep(backoff).await;
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }

        Err(anyhow!(
            "redis command failed after retries: {:?}",
            last_err
        ))
    }
}
