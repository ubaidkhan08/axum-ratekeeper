use anyhow::Result;
use redis::aio::ConnectionManager;

pub async fn create_redis_pool(redis_url: &str) -> Result<ConnectionManager> {
    let client = redis::Client::open(redis_url)?;
    let manager = client.get_tokio_connection_manager().await?;
    Ok(manager)
}
