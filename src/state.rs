use crate::config::Config;
use crate::types::activity::ProcessedActivity;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AppState {
    cache: Arc<DashMap<String, CachedActivity>>,
    licenses: Arc<DashMap<String, CachedLicense>>,
    strava_sessions: Arc<DashMap<String, StravaSession>>,
    config: Arc<Config>,
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
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            licenses: Arc::new(DashMap::new()),
            strava_sessions: Arc::new(DashMap::new()),
            config: Arc::new(config),
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
