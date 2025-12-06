use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub redis_url: String,
    pub bind_addr: SocketAddr,
    pub policy_cache_ttl: Duration,
    pub admin_api_key: Option<String>,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub redis_max_retries: u32,
    pub redis_backoff_ms: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());

        let bind_addr: SocketAddr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()?;

        let policy_cache_ttl = env::var("POLICY_CACHE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(30));

        let admin_api_key = env::var("ADMIN_API_KEY").ok();

        let tls_cert_path = env::var("TLS_CERT_PATH").ok().map(PathBuf::from);
        let tls_key_path = env::var("TLS_KEY_PATH").ok().map(PathBuf::from);

        let redis_max_retries = env::var("REDIS_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);

        let redis_backoff_ms = env::var("REDIS_BACKOFF_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(50);

        Ok(Self {
            redis_url,
            bind_addr,
            policy_cache_ttl,
            admin_api_key,
            tls_cert_path,
            tls_key_path,
            redis_max_retries,
            redis_backoff_ms,
        })
    }
}
