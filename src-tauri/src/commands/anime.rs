use serde::{Deserialize, Serialize};

use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::classifier::{AnimeClassificationInput, AnimeClassifierService};
use crate::services::anime::models::{
    AnimeDetails, AnimeDownloadPayload, AnimeEpisode, AnimeIdentity, AnimeIntroSkipMode,
    AnimeMediaKind, AnimePlaybackRequest, AnimePlaybackSource, AnimeSeason, AnimeSeasonSourceKind,
    AnimeSettings, AnimeSkipTimings, AnimeSourceConfidence, AnimeTranslationMode,
    DiscoveredSubtitle, SkipSegment,
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
pub fn anime_get_details(request: AnimeGetDetailsRequest) -> Result<AnimeGetDetailsResponse, AnimeCommandError> {
    let classifier = AnimeClassifierService::new();
    let classification = classifier.classify(&AnimeClassificationInput {
        has_animation_genre: request.has_animation_genre,
        original_language: request.original_language.clone(),
        origin_countries: request.origin_countries.clone(),
    });

    if !classification.is_anime {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::AnimeNotClassified,
            message: "media item does not meet anime classification threshold".to_string(),
            context: Some(format!(
                "confidence={:.2}; reasons={}"
                , classification.confidence,
                classification.reasons.join(", ")
            )),
        });
    }

    let source_confidence = if classification.confidence >= 0.85 {
        AnimeSourceConfidence::High
    } else if classification.confidence >= 0.7 {
        AnimeSourceConfidence::Medium
    } else {
        AnimeSourceConfidence::Low
    };

    let identity = AnimeIdentity {
        media_kind: AnimeMediaKind::AnimeSeries,
        tmdb_id: request.tmdb_id,
        anilist_id: request.anilist_id,
        mal_id: request.mal_id,
        canonical_title: "Unknown Anime".to_string(),
        romaji_title: None,
        english_title: None,
        native_title: None,
        title_aliases: Vec::new(),
    };

    Ok(AnimeGetDetailsResponse {
        details: AnimeDetails {
            identity,
            overview: None,
            poster_url: None,
            backdrop_url: None,
            genres: Vec::new(),
            score: None,
            release_year: None,
            status: None,
            source_confidence,
        },
    })
}

#[tauri::command]
pub fn anime_get_episode_list(
    request: AnimeGetEpisodeListRequest,
) -> Result<AnimeGetEpisodeListResponse, AnimeCommandError> {
    let season = AnimeSeason {
        season_number: 1,
        title: "Season 1".to_string(),
        episode_count: Some(1),
        source_kind: AnimeSeasonSourceKind::Hybrid,
        episodes: vec![AnimeEpisode {
            display_episode_number: 1,
            canonical_episode_number: 1,
            season_number: 1,
            title: Some("Episode 1".to_string()),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            tmdb_reference: None,
            provider_reference: None,
        }],
    };

    Ok(AnimeGetEpisodeListResponse {
        identity: request.identity,
        seasons: vec![season],
    })
}

#[tauri::command]
pub fn anime_resolve_playback(
    request: AnimePlaybackRequest,
) -> Result<AnimeResolvePlaybackResponse, AnimeCommandError> {
    let source = AnimePlaybackSource {
        provider: crate::services::anime::models::AnimeProvider::Unknown,
        playback_kind: crate::services::anime::models::AnimePlaybackKind::WebviewRemote,
        url: "".to_string(),
        referer: None,
        subtitle_candidates: vec![DiscoveredSubtitle {
            language: "en".to_string(),
            label: Some("English".to_string()),
            url: None,
        }],
        is_downloadable: false,
    };

    Ok(AnimeResolvePlaybackResponse { request, source })
}

#[tauri::command]
pub fn anime_get_skip_timings(
    request: AnimeGetSkipTimingsRequest,
) -> Result<AnimeGetSkipTimingsResponse, AnimeCommandError> {
    Ok(AnimeGetSkipTimingsResponse {
        timings: AnimeSkipTimings {
            mal_id: request.mal_id,
            episode_number: request.episode_number,
            segments: Vec::new(),
        },
    })
}

#[tauri::command]
pub fn anime_set_translation_mode(
    request: AnimeSetTranslationModeRequest,
) -> Result<AnimeSetTranslationModeResponse, AnimeCommandError> {
    Ok(AnimeSetTranslationModeResponse {
        settings: AnimeSettings {
            default_translation_mode: request.translation_mode,
            intro_skip_mode: AnimeIntroSkipMode::Off,
            preferred_subtitle_language: "en".to_string(),
        },
    })
}

#[tauri::command]
pub fn anime_prepare_download(
    request: AnimePrepareDownloadRequest,
) -> Result<AnimePrepareDownloadResponse, AnimeCommandError> {
    let playback_url = request.source.url.clone();

    Ok(AnimePrepareDownloadResponse {
        payload: AnimeDownloadPayload {
            media_name: playback_url.clone(),
            tmdb_id: request.request.anime_id.tmdb_id,
            season_number: request.request.season_number,
            episode_number: request.request.episode_number,
            playback_url,
            referer: request.source.referer,
            subtitle_candidates: Vec::new(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::anime::errors::AnimeErrorCategory;
    use crate::services::anime::models::{
        AnimePlaybackKind, AnimeProvider, AnimeSourceConfidence, SkipSegmentKind,
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
}
