use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub max_file_size: usize,
    pub cache_ttl: Duration,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);

        let max_file_size_mb = std::env::var("MAX_FILE_SIZE_MB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(25);

        let cache_ttl_seconds = std::env::var("CACHE_TTL_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);

        Self {
            port,
            max_file_size: max_file_size_mb * 1024 * 1024,
            cache_ttl: Duration::from_secs(cache_ttl_seconds),
        }
    }
}
