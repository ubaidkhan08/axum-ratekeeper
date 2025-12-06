pub const TOKEN_BUCKET_LUA: &str = r#"
local key = KEYS[1]

local capacity = tonumber(ARGV[1])
local rate = tonumber(ARGV[2])
local now_ms = tonumber(ARGV[3])
local cost = tonumber(ARGV[4])
local ttl = tonumber(ARGV[5])

local data = redis.call('HMGET', key, 'tokens', 'last_refill')
local tokens = tonumber(data[1])
local last_refill = tonumber(data[2])

if tokens == nil or last_refill == nil then
    tokens = capacity
    last_refill = now_ms
end

local elapsed_ms = now_ms - last_refill
if elapsed_ms < 0 then
    elapsed_ms = 0
end

local add = math.floor((elapsed_ms / 1000.0) * rate)

if add > 0 then
    tokens = math.min(capacity, tokens + add)
    last_refill = now_ms
end

local allowed = 0
local retry_after_ms = 0

if tokens >= cost then
    allowed = 1
    tokens = tokens - cost
    retry_after_ms = 0
else
    allowed = 0
    local deficit = cost - tokens
    retry_after_ms = math.ceil((deficit / rate) * 1000)
end

redis.call('HMSET', key, 'tokens', tokens, 'last_refill', last_refill)
redis.call('EXPIRE', key, ttl)

return { allowed, tokens, retry_after_ms }
"#;
