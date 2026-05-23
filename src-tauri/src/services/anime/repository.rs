use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::models::{AnimeDetails, AnimeIdentity, AnimeSettings, AnimeSkipTimings, AnimeTranslationMode};

#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub fetched_at: SystemTime,
    pub ttl: Duration,
}

impl<T> CacheEntry<T> {
    pub fn is_expired(&self, now: SystemTime) -> bool {
        now.duration_since(self.fetched_at)
            .map(|elapsed| elapsed > self.ttl)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnimeRepository {
    identity_cache: HashMap<String, AnimeIdentity>,
    details_cache: HashMap<String, AnimeDetails>,
    translation_preferences: HashMap<String, AnimeTranslationMode>,
    settings: Option<AnimeSettings>,
}

impl AnimeRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_identity(&mut self, identity_key: &str, identity: AnimeIdentity) {
        self.identity_cache.insert(identity_key.to_string(), identity);
    }

    pub fn get_identity(&self, identity_key: &str) -> Option<&AnimeIdentity> {
        self.identity_cache.get(identity_key)
    }

    pub fn upsert_details(&mut self, identity_key: &str, details: AnimeDetails) {
        self.details_cache.insert(identity_key.to_string(), details);
    }

    pub fn get_details(&self, identity_key: &str) -> Option<&AnimeDetails> {
        self.details_cache.get(identity_key)
    }

    pub fn set_translation_preference(&mut self, identity_key: &str, mode: AnimeTranslationMode) {
        self.translation_preferences
            .insert(identity_key.to_string(), mode);
    }

    pub fn get_translation_preference(&self, identity_key: &str) -> Option<&AnimeTranslationMode> {
        self.translation_preferences.get(identity_key)
    }

    pub fn set_settings(&mut self, settings: AnimeSettings) {
        self.settings = Some(settings);
    }

    pub fn get_settings(&self) -> Option<&AnimeSettings> {
        self.settings.as_ref()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnimeCacheRepository {
    anilist_metadata: HashMap<i64, CacheEntry<AnimeDetails>>,
    provider_lookup: HashMap<String, CacheEntry<String>>,
    skip_timings: HashMap<String, CacheEntry<AnimeSkipTimings>>,
}

impl AnimeCacheRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put_anilist_metadata(&mut self, anilist_id: i64, entry: CacheEntry<AnimeDetails>) {
        self.anilist_metadata.insert(anilist_id, entry);
    }

    pub fn get_anilist_metadata(
        &self,
        anilist_id: i64,
        now: SystemTime,
    ) -> Option<&AnimeDetails> {
        self.anilist_metadata
            .get(&anilist_id)
            .filter(|entry| !entry.is_expired(now))
            .map(|entry| &entry.value)
    }

    pub fn put_provider_lookup(&mut self, cache_key: &str, entry: CacheEntry<String>) {
        self.provider_lookup.insert(cache_key.to_string(), entry);
    }

    pub fn get_provider_lookup(&self, cache_key: &str, now: SystemTime) -> Option<&str> {
        self.provider_lookup
            .get(cache_key)
            .filter(|entry| !entry.is_expired(now))
            .map(|entry| entry.value.as_str())
    }

    pub fn put_skip_timings(&mut self, cache_key: &str, entry: CacheEntry<AnimeSkipTimings>) {
        self.skip_timings.insert(cache_key.to_string(), entry);
    }

    pub fn get_skip_timings(&self, cache_key: &str, now: SystemTime) -> Option<&AnimeSkipTimings> {
        self.skip_timings
            .get(cache_key)
            .filter(|entry| !entry.is_expired(now))
            .map(|entry| &entry.value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnimeProgressRepository {
    by_episode_key: HashMap<String, AnimeEpisodeProgress>,
}

#[derive(Debug, Clone)]
pub struct AnimeEpisodeProgress {
    pub canonical_episode_key: String,
    pub identity_key: String,
    pub progress_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub watched_completed: bool,
    pub updated_at: SystemTime,
}

impl AnimeProgressRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_progress(&mut self, progress: AnimeEpisodeProgress) {
        self.by_episode_key
            .insert(progress.canonical_episode_key.clone(), progress);
    }

    pub fn get_progress(&self, canonical_episode_key: &str) -> Option<&AnimeEpisodeProgress> {
        self.by_episode_key.get(canonical_episode_key)
    }
}
