use crate::config::Config;
use crate::types::activity::ProcessedActivity;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct AppState {
    cache: Arc<DashMap<String, CachedActivity>>,
    licenses: Arc<DashMap<String, CachedLicense>>,
    strava_sessions: Arc<DashMap<String, StravaSession>>,
    config: Arc<Config>,
    video_export_semaphore: Arc<Semaphore>,
    video_export_rate_limiter: Arc<VideoExportRateLimiter>,
}

struct CachedActivity {
    activity: ProcessedActivity,
    inserted_at: Instant,
}

#[derive(Clone)]
pub struct CachedLicense {
    pub token: String,
    pub email: String,
    pub is_pro: bool,
    pub expires_at: Instant,
}

#[derive(Clone)]
pub struct StravaSession {
    pub access_token: String,
    pub athlete_id: Option<u64>,
    pub expires_at: Instant,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let video_export_max_concurrency = config.video_export_max_concurrency.max(1);
        let video_export_rate_limit_window = config.video_export_rate_limit_window;
        let video_export_rate_limit_max_requests = config.video_export_rate_limit_max_requests;
        Self {
            cache: Arc::new(DashMap::new()),
            licenses: Arc::new(DashMap::new()),
            strava_sessions: Arc::new(DashMap::new()),
            config: Arc::new(config),
            video_export_semaphore: Arc::new(Semaphore::new(video_export_max_concurrency)),
            video_export_rate_limiter: Arc::new(VideoExportRateLimiter::new(
                video_export_rate_limit_window,
                video_export_rate_limit_max_requests,
            )),
        }
    }

    pub fn insert(&self, file_id: String, activity: ProcessedActivity) {
        self.cache.insert(
            file_id,
            CachedActivity {
                activity,
                inserted_at: Instant::now(),
            },
        );
    }

    pub fn get(&self, file_id: &str) -> Option<ProcessedActivity> {
        self.cache.get(file_id).map(|entry| entry.activity.clone())
    }

    pub fn evict_expired(&self, ttl: Duration) {
        let now = Instant::now();
        self.cache
            .retain(|_, cached| now.duration_since(cached.inserted_at) < ttl);
        self.licenses.retain(|_, license| now < license.expires_at);
        self.strava_sessions
            .retain(|_, session| now < session.expires_at);
        tracing::info!("Cache eviction complete. Current size: {}", self.cache.len());
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn video_export_semaphore(&self) -> Arc<Semaphore> {
        self.video_export_semaphore.clone()
    }

    pub fn video_export_rate_limiter(&self) -> Arc<VideoExportRateLimiter> {
        self.video_export_rate_limiter.clone()
    }

    pub fn store_license(&self, license: CachedLicense) {
        self.licenses.insert(license.token.clone(), license);
    }

    pub fn verify_license(&self, token: &str) -> Option<CachedLicense> {
        self.licenses.get(token).and_then(|entry| {
            if Instant::now() < entry.expires_at {
                Some(entry.clone())
            } else {
                None
            }
        })
    }

    pub fn store_strava_session(&self, session_key: String, session: StravaSession) {
        self.strava_sessions.insert(session_key, session);
    }

    pub fn get_strava_session(&self, session_key: &str) -> Option<StravaSession> {
        self.strava_sessions.get(session_key).and_then(|entry| {
            if Instant::now() < entry.expires_at {
                Some(entry.clone())
            } else {
                None
            }
        })
    }
}

pub struct VideoExportRateLimiter {
    window: Duration,
    max_requests: usize,
    requests: DashMap<String, VecDeque<Instant>>,
}

impl VideoExportRateLimiter {
    pub fn new(window: Duration, max_requests: usize) -> Self {
        Self {
            window,
            max_requests,
            requests: DashMap::new(),
        }
    }

    /// Returns `Ok(())` if allowed, otherwise returns the recommended `retry_after_seconds`.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        if self.max_requests == 0 {
            return Ok(());
        }

        let now = Instant::now();
        let mut entry = self
            .requests
            .entry(key.to_string())
            .or_insert_with(VecDeque::new);

        while let Some(front) = entry.front().copied() {
            if now.duration_since(front) > self.window {
                entry.pop_front();
            } else {
                break;
            }
        }

        if entry.len() >= self.max_requests {
            let retry_after = entry
                .front()
                .copied()
                .map(|oldest| {
                    self.window
                        .saturating_sub(now.duration_since(oldest))
                        .as_secs()
                })
                .unwrap_or(self.window.as_secs());
            return Err(retry_after.max(1));
        }

        entry.push_back(now);
        while entry.len() > self.max_requests {
            entry.pop_front();
        }

        Ok(())
    }
}
