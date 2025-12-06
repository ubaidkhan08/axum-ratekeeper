use super::policy::RatePolicy;

pub const GCRA_LUA: &str = r#"
local key = KEYS[1]

local capacity = tonumber(ARGV[1])
local rate = tonumber(ARGV[2])
local now_ms = tonumber(ARGV[3])
local cost = tonumber(ARGV[4])
local ttl = tonumber(ARGV[5])

local interval = 1000.0 / rate
local tat = redis.call('GET', key)

if tat == false or tat == nil then
    tat = now_ms - (capacity * interval)
else
    tat = tonumber(tat)
end

local allow_at = math.max(tat, now_ms)
local new_tat = allow_at + (cost * interval)
local window_limit = now_ms + (capacity * interval)

local allowed = 0
local retry_after_ms = 0

if new_tat <= window_limit then
    allowed = 1
    tat = new_tat
    retry_after_ms = 0
else
    allowed = 0
    retry_after_ms = math.ceil(new_tat - window_limit)
end

local remaining_tokens = math.floor(math.max(0, (window_limit - tat) / interval))

redis.call('SET', key, tat, 'PX', ttl * 1000)

return { allowed, remaining_tokens, retry_after_ms }
"#;

pub fn compute_gcra_ttl(policy: &RatePolicy) -> i64 {
    let window_seconds =
        (2.0 * (policy.capacity as f64 / policy.refill_tokens_per_sec)).ceil() as u64;
    window_seconds.max(60) as i64
}
