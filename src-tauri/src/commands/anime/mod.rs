use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use serde_json::json;

mod catalog;
mod details_commands;
mod playback_commands;
mod progress_commands;
mod resume;

use crate::database::AppDatabase;
use crate::services::anime::classifier::{AnimeClassificationInput, AnimeClassifierService};
use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::mapping::MappingEpisodeInput;
use crate::services::anime::models::{
    AnimeDetails, AnimeDownloadPayload, AnimeIdentity,
    AnimePlaybackRequest, AnimePlaybackSource, AnimeSeason,
    AnimeSettings, AnimeSkipTimings, AnimeTranslationMode,
    SkipSegment,
};
use self::catalog::{fetch_anilist_json, parse_catalog_items};
use self::resume::parse_episode_progress_key;
use self::details_commands::handle_anime_get_details;
use self::playback_commands::{
    handle_get_episode_list, handle_get_skip_timings, handle_prepare_download,
    handle_resolve_playback, handle_set_translation_mode, handle_execute_download,
    handle_cancel_download,
    handle_get_local_playback_source, handle_remove_download_artifacts,
};
use self::progress_commands::{
    handle_get_resume_progress, handle_set_last_episode, handle_update_progress,
};

pub const EVENT_ANIME_PLAYBACK_READY: &str = "anime-playback-ready";
pub const EVENT_ANIME_PLAYBACK_FAILED: &str = "anime-playback-failed";
pub const EVENT_ANIME_PROGRESS_UPDATED: &str = "anime-progress-updated";
pub const EVENT_ANIME_SKIP_SEGMENT_ACTIVE: &str = "anime-skip-segment-active";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetDetailsRequest {
    pub tmdb_id: Option<i64>,
    pub anilist_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Option<Vec<String>>,
    pub release_year: Option<i32>,
    pub status: Option<String>,
    pub has_animation_genre: bool,
    pub original_language: Option<String>,
    pub origin_countries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetDetailsResponse {
    pub details: AnimeDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetEpisodeListRequest {
    pub identity: AnimeIdentity,
    pub is_movie: bool,
    pub tmdb_episodes: Vec<MappingEpisodeInput>,
    pub anilist_episode_count: Option<i32>,
    pub released_episode_count: Option<i32>,
    pub preferred_season_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetEpisodeListResponse {
    pub identity: AnimeIdentity,
    pub seasons: Vec<AnimeSeason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeResolvePlaybackResponse {
    pub request: AnimePlaybackRequest,
    pub source: AnimePlaybackSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetSkipTimingsRequest {
    pub mal_id: i64,
    pub episode_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetSkipTimingsResponse {
    pub timings: AnimeSkipTimings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSetTranslationModeRequest {
    pub identity: AnimeIdentity,
    pub translation_mode: AnimeTranslationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSetTranslationModeResponse {
    pub settings: AnimeSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePrepareDownloadRequest {
    pub request: AnimePlaybackRequest,
    pub source: AnimePlaybackSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePrepareDownloadResponse {
    pub payload: AnimeDownloadPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeExecuteDownloadRequest {
    pub download_id: String,
    pub payload: AnimeDownloadPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeExecuteDownloadResponse {
    pub output_path: String,
    pub bytes_downloaded: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeCancelDownloadRequest {
    pub download_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeDownloadsEnqueueRequest {
    pub download_id: String,
    pub payload: AnimeDownloadPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeDownloadsListResponse {
    pub downloads: Vec<crate::services::downloads::DownloadRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeRemoveDownloadArtifactsRequest {
    pub payload: AnimeDownloadPayload,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetLocalPlaybackSourceRequest {
    pub request: AnimePlaybackRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeGetLocalPlaybackSourceResponse {
    pub source: Option<AnimePlaybackSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeCatalogItem {
    pub anilist_id: i64,
    pub mal_id: Option<i64>,
    pub title: String,
    pub poster_url: Option<String>,
    pub year: Option<i32>,
    pub episodes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeLatestRequest {
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeLatestResponse {
    pub items: Vec<AnimeCatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSearchRequest {
    pub query: String,
    pub limit: Option<i32>,
    pub has_animation_genre: Option<bool>,
    pub original_language: Option<String>,
    pub origin_countries: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSearchResponse {
    pub items: Vec<AnimeCatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeResumeProgressRequest {
    pub identity: AnimeIdentity,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeResumeProgressResponse {
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub progress_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub watched_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeUpdateProgressRequest {
    pub identity: AnimeIdentity,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub progress_seconds: f64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSetLastEpisodeRequest {
    pub identity: AnimeIdentity,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePlaybackReadyEventPayload {
    pub identity: AnimeIdentity,
    pub source: AnimePlaybackSource,
    pub translation_mode: AnimeTranslationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePlaybackFailedEventPayload {
    pub error: AnimeCommandError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeProgressUpdatedEventPayload {
    pub identity: AnimeIdentity,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub progress_seconds: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeSkipSegmentActiveEventPayload {
    pub identity: AnimeIdentity,
    pub segment: SkipSegment,
}

#[tauri::command]
pub async fn anime_get_latest(
    request: AnimeLatestRequest,
) -> Result<AnimeLatestResponse, AnimeCommandError> {
    let limit = request.limit.unwrap_or(20).clamp(1, 50);
    let query = r#"
      query ($page:Int, $perPage:Int) {
        Page(page:$page, perPage:$perPage) {
          media(type: ANIME, sort: TRENDING_DESC, status_not: NOT_YET_RELEASED) {
            id
            idMal
            episodes
            seasonYear
            title { romaji english }
            coverImage { large }
          }
        }
      }
    "#;

    let payload = json!({ "query": query, "variables": { "page": 1, "perPage": limit } });
    let value = fetch_anilist_json(payload).await?;
    Ok(AnimeLatestResponse {
        items: parse_catalog_items(value),
    })
}

#[tauri::command]
pub async fn anime_search(
    request: AnimeSearchRequest,
) -> Result<AnimeSearchResponse, AnimeCommandError> {
    if request.has_animation_genre.is_some()
        || request.original_language.is_some()
        || request.origin_countries.is_some()
    {
        let classifier = AnimeClassifierService::new();
        let classification = classifier.classify(&AnimeClassificationInput {
            has_animation_genre: request.has_animation_genre.unwrap_or(false),
            original_language: request.original_language.clone(),
            origin_countries: request.origin_countries.clone().unwrap_or_default(),
        });

        if !classification.is_anime {
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::AnimeNotClassified,
                message: "search request does not meet anime classification threshold".to_string(),
                context: Some(format!(
                    "confidence={:.2}; reasons={}",
                    classification.confidence,
                    classification.reasons.join(", ")
                )),
            });
        }
    }

    let limit = request.limit.unwrap_or(12).clamp(1, 30);
    let query = r#"
      query ($page:Int, $perPage:Int, $search:String) {
        Page(page:$page, perPage:$perPage) {
          media(type: ANIME, search: $search, sort: SEARCH_MATCH) {
            id
            idMal
            episodes
            seasonYear
            title { romaji english }
            coverImage { large }
          }
        }
      }
    "#;

    let payload = json!({
        "query": query,
        "variables": { "page": 1, "perPage": limit, "search": request.query }
    });
    let value = fetch_anilist_json(payload).await?;
    Ok(AnimeSearchResponse {
        items: parse_catalog_items(value),
    })
}

#[tauri::command]
pub async fn anime_get_resume_progress(
    request: AnimeResumeProgressRequest,
    db: State<'_, AppDatabase>,
) -> Result<Option<AnimeResumeProgressResponse>, AnimeCommandError> {
    handle_get_resume_progress(request, db).await
}


#[tauri::command]
pub async fn anime_update_progress(
    request: AnimeUpdateProgressRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    handle_update_progress(request, db).await
}

#[tauri::command]
pub async fn anime_set_last_episode(
    request: AnimeSetLastEpisodeRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    handle_set_last_episode(request, db).await
}

#[tauri::command]
pub async fn anime_get_details(
    request: AnimeGetDetailsRequest,
    db: State<'_, AppDatabase>,
) -> Result<AnimeGetDetailsResponse, AnimeCommandError> {
    handle_anime_get_details(request, db).await
}

#[tauri::command]
pub fn anime_get_episode_list(
    request: AnimeGetEpisodeListRequest,
) -> Result<AnimeGetEpisodeListResponse, AnimeCommandError> {
    handle_get_episode_list(request)
}

#[tauri::command]
pub async fn anime_resolve_playback(
    request: AnimePlaybackRequest,
    app: AppHandle,
    db: State<'_, AppDatabase>,
) -> Result<AnimeResolvePlaybackResponse, AnimeCommandError> {
    handle_resolve_playback(request, app, db).await
}

#[tauri::command]
pub async fn anime_get_skip_timings(
    request: AnimeGetSkipTimingsRequest,
    app: AppHandle,
    db: State<'_, AppDatabase>,
) -> Result<AnimeGetSkipTimingsResponse, AnimeCommandError> {
    handle_get_skip_timings(request, app, db).await
}

#[tauri::command]
pub fn anime_set_translation_mode(
    request: AnimeSetTranslationModeRequest,
) -> Result<AnimeSetTranslationModeResponse, AnimeCommandError> {
    handle_set_translation_mode(request)
}

#[tauri::command]
pub fn anime_prepare_download(
    request: AnimePrepareDownloadRequest,
) -> Result<AnimePrepareDownloadResponse, AnimeCommandError> {
    handle_prepare_download(request)
}

#[tauri::command]
pub async fn anime_execute_download(
    request: AnimeExecuteDownloadRequest,
) -> Result<AnimeExecuteDownloadResponse, AnimeCommandError> {
    handle_execute_download(request).await
}

#[tauri::command]
pub fn anime_cancel_download(
    request: AnimeCancelDownloadRequest,
) -> Result<(), AnimeCommandError> {
    handle_cancel_download(request)
}

#[tauri::command]
pub fn anime_remove_download_artifacts(
    request: AnimeRemoveDownloadArtifactsRequest,
) -> Result<(), AnimeCommandError> {
    handle_remove_download_artifacts(request)
}

#[tauri::command]
pub async fn anime_downloads_enqueue(
    request: AnimeDownloadsEnqueueRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    crate::services::downloads::enqueue(&db.0, &request.download_id, &request.payload).await
}

#[tauri::command]
pub async fn anime_downloads_list(
    db: State<'_, AppDatabase>,
) -> Result<AnimeDownloadsListResponse, AnimeCommandError> {
    let downloads = crate::services::downloads::list(&db.0).await?;
    Ok(AnimeDownloadsListResponse { downloads })
}

#[tauri::command]
pub async fn anime_downloads_cancel(
    request: AnimeCancelDownloadRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    crate::services::downloads::cancel(&db.0, &request.download_id).await
}

#[tauri::command]
pub async fn anime_downloads_remove(
    request: AnimeCancelDownloadRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    if let Some(payload) = crate::services::downloads::remove(&db.0, &request.download_id).await? {
        let _ = handle_remove_download_artifacts(AnimeRemoveDownloadArtifactsRequest {
            payload,
            output_path: None,
        });
    }
    Ok(())
}

#[tauri::command]
pub fn anime_get_local_playback_source(
    request: AnimeGetLocalPlaybackSourceRequest,
) -> Result<AnimeGetLocalPlaybackSourceResponse, AnimeCommandError> {
    handle_get_local_playback_source(request)
}

fn build_download_file_name(
    canonical_title: &str,
    season_number: Option<i32>,
    episode_number: Option<i32>,
) -> String {
    let title = canonical_title
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    match (season_number, episode_number) {
        (Some(season), Some(episode)) => format!("{title}_S{season:02}E{episode:02}.mp4"),
        _ => format!("{title}.mp4"),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::resume::parse_key_number;
    use crate::services::anime::errors::AnimeErrorCategory;
    use crate::services::anime::models::{
        AnimeMediaKind, AnimePlaybackKind, AnimeProvider, AnimeSourceConfidence, SkipSegmentKind,
    };

    #[test]
    fn serializes_anime_identity_in_camel_case() {
        let identity = AnimeIdentity {
            media_kind: AnimeMediaKind::AnimeSeries,
            tmdb_id: Some(1),
            anilist_id: Some(2),
            mal_id: Some(3),
            canonical_title: "Test".to_string(),
            romaji_title: None,
            english_title: None,
            native_title: None,
            title_aliases: vec!["Alias".to_string()],
        };

        let value = serde_json::to_value(identity).expect("identity should serialize");

        assert!(value.get("mediaKind").is_some());
        assert!(value.get("tmdbId").is_some());
        assert!(value.get("canonicalTitle").is_some());
        assert!(value.get("titleAliases").is_some());
    }

    #[test]
    fn serializes_command_responses_in_camel_case() {
        let response = AnimeGetDetailsResponse {
            details: AnimeDetails {
                identity: AnimeIdentity {
                    media_kind: AnimeMediaKind::AnimeMovie,
                    tmdb_id: Some(99),
                    anilist_id: None,
                    mal_id: None,
                    canonical_title: "Movie".to_string(),
                    romaji_title: None,
                    english_title: None,
                    native_title: None,
                    title_aliases: Vec::new(),
                },
                overview: None,
                poster_url: None,
                backdrop_url: None,
                genres: vec![],
                score: None,
                total_episodes: None,
                released_episode_count: None,
                release_year: None,
                status: None,
                source_confidence: AnimeSourceConfidence::Unknown,
            },
        };

        let value = serde_json::to_value(response).expect("response should serialize");

        assert!(value.get("details").is_some());
        assert!(value["details"].get("sourceConfidence").is_some());
    }

    #[test]
    fn serializes_error_categories_and_payloads() {
        let payload = AnimePlaybackFailedEventPayload {
            error: AnimeCommandError {
                category: AnimeErrorCategory::TranslationUnavailable,
                message: "dub unavailable".to_string(),
                context: Some("episode 4".to_string()),
            },
        };

        let value = serde_json::to_value(payload).expect("error payload should serialize");

        assert_eq!(value["error"]["category"], "translationUnavailable");
        assert_eq!(value["error"]["message"], "dub unavailable");
    }

    #[test]
    fn serializes_event_payload_shapes() {
        let identity = AnimeIdentity {
            media_kind: AnimeMediaKind::AnimeSeries,
            tmdb_id: Some(20),
            anilist_id: Some(30),
            mal_id: Some(40),
            canonical_title: "Event Test".to_string(),
            romaji_title: None,
            english_title: None,
            native_title: None,
            title_aliases: vec![],
        };

        let ready_payload = AnimePlaybackReadyEventPayload {
            identity: identity.clone(),
            source: AnimePlaybackSource {
                provider: AnimeProvider::Unknown,
                playback_kind: AnimePlaybackKind::WebviewRemote,
                url: "https://example.com/stream".to_string(),
                referer: None,
                subtitle_candidates: vec![],
                is_downloadable: true,
            },
            translation_mode: AnimeTranslationMode::Sub,
        };

        let skip_payload = AnimeSkipSegmentActiveEventPayload {
            identity,
            segment: SkipSegment {
                kind: SkipSegmentKind::Intro,
                start_seconds: 12.5,
                end_seconds: 85.0,
            },
        };

        let ready_value =
            serde_json::to_value(ready_payload).expect("ready payload should serialize");
        let skip_value = serde_json::to_value(skip_payload).expect("skip payload should serialize");

        assert_eq!(ready_value["translationMode"], "sub");
        assert!(ready_value["source"].get("playbackKind").is_some());
        assert_eq!(skip_value["segment"]["kind"], "intro");
    }

    #[test]
    fn prepare_download_returns_complete_payload() {
        let request = AnimePrepareDownloadRequest {
            request: AnimePlaybackRequest {
                anime_id: AnimeIdentity {
                    media_kind: AnimeMediaKind::AnimeSeries,
                    tmdb_id: Some(11),
                    anilist_id: Some(22),
                    mal_id: Some(33),
                    canonical_title: "Attack on Titan".to_string(),
                    romaji_title: None,
                    english_title: None,
                    native_title: None,
                    title_aliases: vec![],
                },
                translation_mode: AnimeTranslationMode::Sub,
                movie: false,
                season_number: Some(1),
                episode_number: Some(2),
                resume_seconds: Some(10),
            },
            source: AnimePlaybackSource {
                provider: AnimeProvider::Gogoanime,
                playback_kind: AnimePlaybackKind::WebviewRemote,
                url: "https://stream/sub.m3u8".to_string(),
                referer: Some("https://provider.example".to_string()),
                subtitle_candidates: vec![crate::services::anime::models::DiscoveredSubtitle {
                    language: "en".to_string(),
                    label: Some("English".to_string()),
                    url: Some("https://sub.example/en.vtt".to_string()),
                }],
                is_downloadable: true,
            },
        };

        let response = anime_prepare_download(request).expect("payload should build");
        assert_eq!(response.payload.file_name, "Attack_on_Titan_S01E02.mp4");
        assert!(!response.payload.request_headers.is_empty());
        assert_eq!(response.payload.subtitle_candidates.len(), 1);
    }

    #[test]
    fn prepare_download_fails_when_source_url_missing() {
        let request = AnimePrepareDownloadRequest {
            request: AnimePlaybackRequest {
                anime_id: AnimeIdentity {
                    media_kind: AnimeMediaKind::AnimeSeries,
                    tmdb_id: Some(11),
                    anilist_id: Some(22),
                    mal_id: Some(33),
                    canonical_title: "Attack on Titan".to_string(),
                    romaji_title: None,
                    english_title: None,
                    native_title: None,
                    title_aliases: vec![],
                },
                translation_mode: AnimeTranslationMode::Sub,
                movie: false,
                season_number: Some(1),
                episode_number: Some(2),
                resume_seconds: Some(10),
            },
            source: AnimePlaybackSource {
                provider: AnimeProvider::Gogoanime,
                playback_kind: AnimePlaybackKind::WebviewRemote,
                url: "  ".to_string(),
                referer: None,
                subtitle_candidates: vec![],
                is_downloadable: true,
            },
        };

        let error = anime_prepare_download(request).expect_err("should fail on empty url");
        assert!(matches!(error.category, AnimeErrorCategory::PlayableSourceMissing));
    }

    #[test]
    fn parses_episode_progress_key_with_some_values() {
        let (season, episode) = parse_episode_progress_key(
            "tmdb:Some(1)|anilist:Some(2)|mal:Some(3)|season:Some(4)|episode:Some(7)",
        );
        assert_eq!(season, Some(4));
        assert_eq!(episode, Some(7));
    }

    #[test]
    fn parses_episode_progress_key_with_none_values() {
        let (season, episode) = parse_episode_progress_key(
            "tmdb:Some(1)|anilist:Some(2)|mal:Some(3)|season:None|episode:None",
        );
        assert_eq!(season, None);
        assert_eq!(episode, None);
    }

    #[test]
    fn parse_key_number_handles_plain_and_wrapped_numbers() {
        assert_eq!(parse_key_number("7"), Some(7));
        assert_eq!(parse_key_number("Some(12)"), Some(12));
        assert_eq!(parse_key_number("None"), None);
        assert_eq!(parse_key_number("Some(x)"), None);
    }
}
