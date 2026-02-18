use crate::types::activity::ProcessedActivity;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct AppState {
    cache: Arc<DashMap<String, CachedActivity>>,
}

struct CachedActivity {
    activity: ProcessedActivity,
    inserted_at: Instant,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
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
        self.cache.retain(|_, cached| {
            now.duration_since(cached.inserted_at) < ttl
        });
        tracing::info!("Cache eviction complete. Current size: {}", self.cache.len());
    }
}
