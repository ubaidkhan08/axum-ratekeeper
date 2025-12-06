use super::policy::RatePolicy;
use crate::rate_limiter::lua_scripts::TOKEN_BUCKET_LUA;

pub fn compute_ttl(policy: &RatePolicy) -> i64 {
    let window_seconds =
        (2.0 * (policy.capacity as f64 / policy.refill_tokens_per_sec)).ceil() as u64;
    let ttl = window_seconds.max(60);
    ttl as i64
}

pub fn token_bucket_script() -> redis::Script {
    redis::Script::new(TOKEN_BUCKET_LUA)
}
