use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub max_file_size: usize,
    pub cache_ttl: Duration,
    pub jwt_secret: String,
    pub app_base_url: String,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub stripe_price_id: Option<String>,
    pub strava_client_id: Option<String>,
    pub strava_client_secret: Option<String>,
    pub strava_redirect_uri: Option<String>,
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

        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev-insecure-change-me".to_string());

        let app_base_url = std::env::var("APP_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        Self {
            port,
            max_file_size: max_file_size_mb * 1024 * 1024,
            cache_ttl: Duration::from_secs(cache_ttl_seconds),
            jwt_secret,
            app_base_url,
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            stripe_price_id: std::env::var("STRIPE_PRICE_ID").ok(),
            strava_client_id: std::env::var("STRAVA_CLIENT_ID").ok(),
            strava_client_secret: std::env::var("STRAVA_CLIENT_SECRET").ok(),
            strava_redirect_uri: std::env::var("STRAVA_REDIRECT_URI").ok(),
        }
    }
}
