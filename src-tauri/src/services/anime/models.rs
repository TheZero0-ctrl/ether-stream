use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimeMediaKind {
    AnimeSeries,
    AnimeMovie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeIdentity {
    pub media_kind: AnimeMediaKind,
    pub tmdb_id: Option<i64>,
    pub anilist_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub canonical_title: String,
    pub romaji_title: Option<String>,
    pub english_title: Option<String>,
    pub native_title: Option<String>,
    pub title_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimeSourceConfidence {
    High,
    Medium,
    Low,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeDetails {
    pub identity: AnimeIdentity,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
    pub score: Option<f32>,
    pub total_episodes: Option<i32>,
    pub released_episode_count: Option<i32>,
    pub release_year: Option<i32>,
    pub status: Option<String>,
    pub source_confidence: AnimeSourceConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimeSeasonSourceKind {
    Tmdb,
    Anilist,
    Hybrid,
    ProviderDerived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSeason {
    pub season_number: i32,
    pub title: String,
    pub episode_count: Option<i32>,
    pub source_kind: AnimeSeasonSourceKind,
    pub episodes: Vec<AnimeEpisode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmdbEpisodeRef {
    pub season_number: i32,
    pub episode_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimeProvider {
    Gogoanime,
    Zoro,
    AnimePahe,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderEpisodeRef {
    pub provider: AnimeProvider,
    pub provider_show_id: Option<String>,
    pub provider_episode_token: Option<String>,
    pub provider_season_number: Option<i32>,
    pub provider_episode_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeEpisode {
    pub display_episode_number: i32,
    pub canonical_episode_number: i32,
    pub season_number: i32,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<i32>,
    pub air_date: Option<String>,
    pub tmdb_reference: Option<TmdbEpisodeRef>,
    pub provider_reference: Option<ProviderEpisodeRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimeTranslationMode {
    Sub,
    Dub,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePlaybackRequest {
    pub anime_id: AnimeIdentity,
    pub translation_mode: AnimeTranslationMode,
    pub movie: bool,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub resume_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimePlaybackKind {
    WebviewRemote,
    LocalProxy,
    DirectVideo,
    Hls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredSubtitle {
    pub language: String,
    pub label: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePlaybackSource {
    pub provider: AnimeProvider,
    pub playback_kind: AnimePlaybackKind,
    pub url: String,
    pub referer: Option<String>,
    pub subtitle_candidates: Vec<DiscoveredSubtitle>,
    pub is_downloadable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkipSegmentKind {
    Intro,
    Outro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipSegment {
    pub kind: SkipSegmentKind,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSkipTimings {
    pub mal_id: i64,
    pub episode_number: i32,
    pub segments: Vec<SkipSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSubtitle {
    pub language: String,
    pub label: Option<String>,
    pub file_path: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeDownloadPayload {
    pub media_name: String,
    pub tmdb_id: Option<i64>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub identity_key: String,
    pub file_name: String,
    pub playback_kind: AnimePlaybackKind,
    pub playback_url: String,
    pub referer: Option<String>,
    pub request_headers: Vec<(String, String)>,
    pub subtitle_candidates: Vec<ResolvedSubtitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimeIntroSkipMode {
    Off,
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSettings {
    pub default_translation_mode: AnimeTranslationMode,
    pub intro_skip_mode: AnimeIntroSkipMode,
    pub preferred_subtitle_language: String,
}
