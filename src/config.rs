use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub max_file_size: usize,
    pub cache_ttl: Duration,
    pub jwt_secret: String,
    pub app_base_url: String,

    // Video export protection (hot path)
    pub video_export_max_concurrency: usize,
    pub video_export_queue_timeout: Duration,
    pub video_export_timeout: Duration,
    pub video_export_rate_limit_window: Duration,
    pub video_export_rate_limit_max_requests: usize,

    pub stripe_allow_mock: bool,
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

        let video_export_max_concurrency = std::env::var("VIDEO_EXPORT_MAX_CONCURRENCY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);

        let video_export_queue_timeout_seconds = std::env::var("VIDEO_EXPORT_QUEUE_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);

        let video_export_timeout_seconds = std::env::var("VIDEO_EXPORT_TIMEOUT_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);

        let video_export_rate_limit_window_seconds =
            std::env::var("VIDEO_EXPORT_RATE_LIMIT_WINDOW_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);

        let video_export_rate_limit_max_requests = std::env::var("VIDEO_EXPORT_RATE_LIMIT_MAX_REQUESTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);

        let stripe_allow_mock = if cfg!(debug_assertions) {
            std::env::var("STRIPE_ALLOW_MOCK")
                .ok()
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or(true)
        } else {
            false
        };

        Self {
            port,
            max_file_size: max_file_size_mb * 1024 * 1024,
            cache_ttl: Duration::from_secs(cache_ttl_seconds),
            jwt_secret,
            app_base_url,
            video_export_max_concurrency,
            video_export_queue_timeout: Duration::from_secs(video_export_queue_timeout_seconds),
            video_export_timeout: Duration::from_secs(video_export_timeout_seconds),
            video_export_rate_limit_window: Duration::from_secs(video_export_rate_limit_window_seconds),
            video_export_rate_limit_max_requests,
            stripe_allow_mock,
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            stripe_price_id: std::env::var("STRIPE_PRICE_ID").ok(),
            strava_client_id: std::env::var("STRAVA_CLIENT_ID").ok(),
            strava_client_secret: std::env::var("STRAVA_CLIENT_SECRET").ok(),
            strava_redirect_uri: std::env::var("STRAVA_REDIRECT_URI").ok(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3000,
            max_file_size: 25 * 1024 * 1024,
            cache_ttl: Duration::from_secs(3600),
            jwt_secret: "dev-insecure-change-me".to_string(),
            app_base_url: "http://localhost:3000".to_string(),
            video_export_max_concurrency: 2,
            video_export_queue_timeout: Duration::from_secs(2),
            video_export_timeout: Duration::from_secs(120),
            video_export_rate_limit_window: Duration::from_secs(60),
            video_export_rate_limit_max_requests: 4,
            stripe_allow_mock: cfg!(debug_assertions),
            stripe_secret_key: None,
            stripe_webhook_secret: None,
            stripe_price_id: None,
            strava_client_id: None,
            strava_client_secret: None,
            strava_redirect_uri: None,
        }
    }
}
