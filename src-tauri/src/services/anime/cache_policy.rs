use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AnimeCachePolicy {
    pub anilist_metadata_ttl: Duration,
    pub provider_lookup_ttl: Duration,
    pub skip_timings_ttl: Duration,
}

impl AnimeCachePolicy {
    pub fn production_default() -> Self {
        Self {
            anilist_metadata_ttl: Duration::from_secs(60 * 60 * 24),
            provider_lookup_ttl: Duration::from_secs(60 * 60 * 6),
            skip_timings_ttl: Duration::from_secs(60 * 60 * 24 * 7),
        }
    }
}
