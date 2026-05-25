use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use serde_json::json;

use crate::database::AppDatabase;
use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::classifier::{AnimeClassificationInput, AnimeClassifierService};
use crate::services::anime::metadata::{
    lookup_anilist_candidate, AnilistCandidate, AnilistLookupResult, AnimeMetadataService,
    MetadataEnrichmentError, TmdbMetadataInput,
};
use crate::services::anime::mapping::{AnimeMappingService, MappingEpisodeInput, MappingInput};
use crate::services::anime::resolver::{
    AnimeResolverService, ResolverContext,
};
use crate::services::anime::repository_sqlx::AnimeSqlxRepository;
use crate::services::anime::repository_sqlx::AnimeSqlxProgressRepository;
use crate::services::anime::repository_sqlx::AnimeSqlxCacheRepository;
use crate::services::anime::playback::{AnimePlaybackService, ProgressUpdate};
use crate::services::anime::skip::{AnimeSkipMode, AnimeSkipService};
use crate::services::anime::models::{
    AnimeDetails, AnimeDownloadPayload, AnimeIdentity, AnimeIntroSkipMode, AnimeMediaKind,
    AnimePlaybackRequest, AnimePlaybackSource, AnimeSeason,
    AnimeSettings, AnimeSkipTimings, AnimeSourceConfidence, AnimeTranslationMode,
    SkipSegment,
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
    let identity_key = build_identity_key(&request.identity);
    let repo = AnimeSqlxProgressRepository::new(db.0.clone());

    if request.season_number.is_none() && request.episode_number.is_none() {
        let last = repo
            .get_last_episode(&identity_key)
            .await
            .map_err(|err| AnimeCommandError {
                category: AnimeErrorCategory::PlayableSourceMissing,
                message: "failed to read last episode".to_string(),
                context: Some(err.to_string()),
            })?;

        if let Some((season_number, episode_number)) = last {
            let key = build_episode_progress_key(&request.identity, season_number, episode_number);
            let row = repo
                .get_progress(&key)
                .await
                .map_err(|err| AnimeCommandError {
                    category: AnimeErrorCategory::PlayableSourceMissing,
                    message: "failed to read resume progress".to_string(),
                    context: Some(err.to_string()),
                })?;

            return Ok(Some(match row {
                Some((progress, duration, watched)) => AnimeResumeProgressResponse {
                    season_number,
                    episode_number,
                    progress_seconds: progress,
                    duration_seconds: duration,
                    watched_completed: watched,
                },
                None => AnimeResumeProgressResponse {
                    season_number,
                    episode_number,
                    progress_seconds: 0.0,
                    duration_seconds: None,
                    watched_completed: false,
                },
            }));
        }
    }

    if request.season_number.is_some() || request.episode_number.is_some() {
        let key = build_episode_progress_key(&request.identity, request.season_number, request.episode_number);
        let row = repo
            .get_progress(&key)
            .await
            .map_err(|err| AnimeCommandError {
                category: AnimeErrorCategory::PlayableSourceMissing,
                message: "failed to read resume progress".to_string(),
                context: Some(err.to_string()),
            })?;

        return Ok(row.map(|(progress, duration, watched)| AnimeResumeProgressResponse {
            season_number: request.season_number,
            episode_number: request.episode_number,
            progress_seconds: progress,
            duration_seconds: duration,
            watched_completed: watched,
        }));
    }

    let latest = repo
        .get_latest_progress_for_identity(&identity_key)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to read latest resume progress".to_string(),
            context: Some(err.to_string()),
        })?;

    Ok(latest.map(|(episode_key, progress, duration, watched)| {
        let (season_number, episode_number) = parse_episode_progress_key(&episode_key);
        AnimeResumeProgressResponse {
            season_number,
            episode_number,
            progress_seconds: progress,
            duration_seconds: duration,
            watched_completed: watched,
        }
    }))
}

fn parse_episode_progress_key(key: &str) -> (Option<i32>, Option<i32>) {
    let mut season = None;
    let mut episode = None;

    for part in key.split('|') {
        if let Some(raw) = part.strip_prefix("season:") {
            season = parse_key_number(raw);
        } else if let Some(raw) = part.strip_prefix("episode:") {
            episode = parse_key_number(raw);
        }
    }

    (season, episode)
}

fn parse_key_number(raw: &str) -> Option<i32> {
    let trimmed = raw.trim();
    if trimmed == "None" {
        return None;
    }

    if let Some(inner) = trimmed.strip_prefix("Some(").and_then(|v| v.strip_suffix(')')) {
        return inner.trim().parse::<i32>().ok();
    }

    trimmed.parse::<i32>().ok()
}

#[tauri::command]
pub async fn anime_update_progress(
    request: AnimeUpdateProgressRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    if request.progress_seconds <= 0.0 {
        return Ok(());
    }

    let episode_key = build_episode_progress_key(
        &request.identity,
        request.season_number,
        request.episode_number,
    );
    let identity_key = build_identity_key(&request.identity);
    let repository = AnimeSqlxProgressRepository::new(db.0.clone());

    let watched_completed = request
        .duration_seconds
        .map(|duration| duration > 0.0 && (request.progress_seconds / duration) >= 0.9)
        .unwrap_or(false);

    repository
        .upsert_progress(
            &episode_key,
            &identity_key,
            request.progress_seconds,
            request.duration_seconds,
            watched_completed,
        )
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to persist anime playback progress".to_string(),
            context: Some(err.to_string()),
        })
}

#[tauri::command]
pub async fn anime_set_last_episode(
    request: AnimeSetLastEpisodeRequest,
    db: State<'_, AppDatabase>,
) -> Result<(), AnimeCommandError> {
    let identity_key = build_identity_key(&request.identity);
    let repository = AnimeSqlxProgressRepository::new(db.0.clone());
    repository
        .set_last_episode(&identity_key, request.season_number, request.episode_number)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to persist last selected episode".to_string(),
            context: Some(err.to_string()),
        })
}

#[tauri::command]
pub async fn anime_get_details(
    request: AnimeGetDetailsRequest,
    db: State<'_, AppDatabase>,
) -> Result<AnimeGetDetailsResponse, AnimeCommandError> {
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

    let metadata_service = AnimeMetadataService::new();

    let tmdb_input = TmdbMetadataInput {
        tmdb_id: request.tmdb_id,
        media_kind: AnimeMediaKind::AnimeSeries,
        title: request
            .title
            .unwrap_or_else(|| "Unknown Anime".to_string()),
        overview: request.overview,
        poster_url: request.poster_url,
        backdrop_url: request.backdrop_url,
        genres: request.genres.unwrap_or_default(),
        release_year: request.release_year,
        status: request.status,
    };

    let anilist_lookup = if let Some(anilist_id) = request.anilist_id {
        AnilistLookupResult::Found(AnilistCandidate {
            anilist_id,
            mal_id: request.mal_id,
            canonical_title: tmdb_input.title.clone(),
            romaji_title: None,
            english_title: Some(tmdb_input.title.clone()),
            native_title: None,
            aliases: vec![tmdb_input.title.clone()],
            score: None,
            total_episodes: None,
        })
    } else {
        match lookup_anilist_candidate(&tmdb_input.title).await {
            Ok(Some(candidate)) => AnilistLookupResult::Found(candidate),
            Ok(None) => AnilistLookupResult::Missing,
            Err(error) => {
                return Err(AnimeCommandError {
                    category: AnimeErrorCategory::AnilistMatchMissing,
                    message: "anilist lookup failed".to_string(),
                    context: Some(error),
                });
            }
        }
    };

    match metadata_service.enrich(tmdb_input, anilist_lookup) {
        Ok(output) => Ok(AnimeGetDetailsResponse {
            details: {
                let details = AnimeDetails {
                source_confidence: if classification.confidence >= 0.85 {
                    AnimeSourceConfidence::High
                } else if classification.confidence >= 0.7 {
                    AnimeSourceConfidence::Medium
                } else {
                    AnimeSourceConfidence::Low
                },
                ..output.details
                };

                persist_resolved_identity(&db, &details.identity).await?;
                details
            },
        }),
        Err(MetadataEnrichmentError::MissingMatch) => Err(AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "no AniList match found for anime metadata enrichment".to_string(),
            context: request.tmdb_id.map(|id| format!("tmdb_id={id}")),
        }),
        Err(MetadataEnrichmentError::AmbiguousMatch { candidate_ids }) => Err(AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "multiple AniList candidates found; reconciliation needed".to_string(),
            context: Some(format!("candidate_ids={candidate_ids:?}")),
        }),
    }
}

async fn persist_resolved_identity(
    db: &State<'_, AppDatabase>,
    identity: &AnimeIdentity,
) -> Result<(), AnimeCommandError> {
    if identity.anilist_id.is_none() && identity.mal_id.is_none() {
        return Ok(());
    }

    let identity_key = build_identity_key(identity);
    let repository = AnimeSqlxRepository::new(db.0.clone());

    repository
        .upsert_identity(&identity_key, identity)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "failed to persist resolved anime identity".to_string(),
            context: Some(err.to_string()),
        })
}

fn build_identity_key(identity: &AnimeIdentity) -> String {
    format!(
        "tmdb:{:?}|anilist:{:?}|mal:{:?}",
        identity.tmdb_id, identity.anilist_id, identity.mal_id
    )
}

fn build_episode_progress_key(
    identity: &AnimeIdentity,
    season_number: Option<i32>,
    episode_number: Option<i32>,
) -> String {
    format!(
        "tmdb:{:?}|anilist:{:?}|mal:{:?}|season:{:?}|episode:{:?}",
        identity.tmdb_id, identity.anilist_id, identity.mal_id, season_number, episode_number
    )
}

async fn persist_session_progress(
    db: &State<'_, AppDatabase>,
    session: &crate::services::anime::playback::AnimePlaybackSession,
) -> Result<(), AnimeCommandError> {
    let episode_key = build_episode_progress_key(
        &session.identity,
        session.season_number,
        session.episode_number,
    );
    let identity_key = build_identity_key(&session.identity);
    let repository = AnimeSqlxProgressRepository::new(db.0.clone());

    repository
        .upsert_progress(
            &episode_key,
            &identity_key,
            session.resume_seconds,
            session.duration_seconds,
            session.watched_completed,
        )
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to persist anime playback progress".to_string(),
            context: Some(err.to_string()),
        })
}

#[tauri::command]
pub fn anime_get_episode_list(
    request: AnimeGetEpisodeListRequest,
) -> Result<AnimeGetEpisodeListResponse, AnimeCommandError> {
    let mapper = AnimeMappingService::new();
    let output = mapper.map(MappingInput {
        is_movie: request.is_movie,
        tmdb_episodes: request.tmdb_episodes,
        anilist_episode_count: request.anilist_episode_count,
        provider: None,
    });

    Ok(AnimeGetEpisodeListResponse {
        identity: request.identity,
        seasons: output.seasons,
    })
}

#[tauri::command]
pub async fn anime_resolve_playback(
    request: AnimePlaybackRequest,
    app: AppHandle,
    db: State<'_, AppDatabase>,
) -> Result<AnimeResolvePlaybackResponse, AnimeCommandError> {
    let progress_repository = AnimeSqlxProgressRepository::new(db.0.clone());
    let identity_key = build_identity_key(&request.anime_id);
    progress_repository
        .set_last_episode(&identity_key, request.season_number, request.episode_number)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to persist last selected episode".to_string(),
            context: Some(err.to_string()),
        })?;

    let resolver = AnimeResolverService::new();
    let result = resolver
        .resolve_live(&ResolverContext {
            identity: request.anime_id.clone(),
            season_number: request.season_number,
            episode_number: request.episode_number,
            translation_mode: request.translation_mode.clone(),
            provider: crate::services::anime::models::AnimeProvider::Gogoanime,
            simulate_provider_timeouts: 0,
        })
        .await;

    let result = match result {
        Ok(result) => result,
        Err(error) => {
            let _ = app.emit(
                EVENT_ANIME_PLAYBACK_FAILED,
                AnimePlaybackFailedEventPayload {
                    error: error.clone(),
                },
            );
            return Err(error);
        }
    };

    let source = AnimePlaybackSource {
        is_downloadable: true,
        ..result.source
    };

    let playback_service = AnimePlaybackService::new();
    let session = playback_service.start_session(
        request.anime_id.clone(),
        request.season_number,
        request.episode_number,
        result.selected_translation,
        source.clone(),
        request.resume_seconds,
    );

    let session = playback_service.update_progress(
        &session,
        ProgressUpdate {
            progress_seconds: session.resume_seconds,
            duration_seconds: session.duration_seconds,
        },
    );

    if session.resume_seconds > 0.0 {
        persist_session_progress(&db, &session).await?;
    }

    let _ = app.emit(
        EVENT_ANIME_PLAYBACK_READY,
        AnimePlaybackReadyEventPayload {
            identity: session.identity.clone(),
            source: session.source.clone(),
            translation_mode: session.translation_mode.clone(),
        },
    );

    let _ = app.emit(
        EVENT_ANIME_PROGRESS_UPDATED,
        AnimeProgressUpdatedEventPayload {
            identity: session.identity.clone(),
            season_number: session.season_number,
            episode_number: session.episode_number,
            progress_seconds: session.resume_seconds,
            duration_seconds: session.duration_seconds.unwrap_or(0.0),
        },
    );

    Ok(AnimeResolvePlaybackResponse { request, source })
}

#[tauri::command]
pub async fn anime_get_skip_timings(
    request: AnimeGetSkipTimingsRequest,
    app: AppHandle,
    db: State<'_, AppDatabase>,
) -> Result<AnimeGetSkipTimingsResponse, AnimeCommandError> {
    let cache_repo = AnimeSqlxCacheRepository::new(db.0.clone());

    let cached = cache_repo
        .get_skip_timings(request.mal_id, request.episode_number)
        .await
    .map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "failed to read cached AniSkip timings".to_string(),
        context: Some(err.to_string()),
    })?;

    if let Some(timings) = cached {
        maybe_emit_skip_active(
            &app,
            request.mal_id,
            request.episode_number,
            &timings,
            AnimeSkipMode::Manual,
        );
        return Ok(AnimeGetSkipTimingsResponse { timings });
    }

    let service = AnimeSkipService::new();
    let timings = service
        .fetch_timings(request.mal_id, request.episode_number)
        .await
    .map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "failed to fetch AniSkip timings".to_string(),
        context: Some(err),
    })?;

    let expires_at = "2099-12-31T23:59:59Z";
    let timings_clone = timings.clone();
    cache_repo
        .put_skip_timings(
            request.mal_id,
            request.episode_number,
            &timings_clone,
            expires_at,
        )
        .await
    .map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "failed to cache AniSkip timings".to_string(),
        context: Some(err.to_string()),
    })?;

    maybe_emit_skip_active(&app, request.mal_id, request.episode_number, &timings, AnimeSkipMode::Manual);

    Ok(AnimeGetSkipTimingsResponse { timings })
}

fn maybe_emit_skip_active(
    app: &AppHandle,
    mal_id: i64,
    episode_number: i32,
    timings: &AnimeSkipTimings,
    mode: AnimeSkipMode,
) {
    let skip_service = AnimeSkipService::new();
    if !skip_service.can_emit_active_segment(mode, true, true) {
        return;
    }

    if let Some(segment) = timings.segments.first() {
        let _ = app.emit(
            EVENT_ANIME_SKIP_SEGMENT_ACTIVE,
            AnimeSkipSegmentActiveEventPayload {
                identity: AnimeIdentity {
                    media_kind: AnimeMediaKind::AnimeSeries,
                    tmdb_id: None,
                    anilist_id: None,
                    mal_id: Some(mal_id),
                    canonical_title: format!("MAL-{mal_id}"),
                    romaji_title: None,
                    english_title: None,
                    native_title: None,
                    title_aliases: vec![format!("episode-{episode_number}")],
                },
                segment: segment.clone(),
            },
        );
    }
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
    if playback_url.trim().is_empty() {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "cannot prepare download without playable source url".to_string(),
            context: None,
        });
    }
    let identity_key = build_identity_key(&request.request.anime_id);
    let file_name = build_download_file_name(
        &request.request.anime_id.canonical_title,
        request.request.season_number,
        request.request.episode_number,
    );
    let mut request_headers = Vec::new();
    if let Some(referer) = request.source.referer.clone() {
        request_headers.push(("Referer".to_string(), referer));
    }
    request_headers.push((
        "User-Agent".to_string(),
        "Ether/0.1 AnimeDownloader".to_string(),
    ));

    let subtitle_candidates = request
        .source
        .subtitle_candidates
        .iter()
        .map(|item| crate::services::anime::models::ResolvedSubtitle {
            language: item.language.clone(),
            label: item.label.clone(),
            file_path: None,
            url: item.url.clone(),
        })
        .collect();

    Ok(AnimePrepareDownloadResponse {
        payload: AnimeDownloadPayload {
            media_name: request.request.anime_id.canonical_title.clone(),
            tmdb_id: request.request.anime_id.tmdb_id,
            season_number: request.request.season_number,
            episode_number: request.request.episode_number,
            identity_key,
            file_name,
            playback_url,
            referer: request.source.referer,
            request_headers,
            subtitle_candidates,
        },
    })
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

async fn fetch_anilist_json(payload: serde_json::Value) -> Result<serde_json::Value, AnimeCommandError> {
    let response = reqwest::Client::new()
        .post("https://graphql.anilist.co")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "anilist request failed".to_string(),
            context: Some(err.to_string()),
        })?;

    if !response.status().is_success() {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "anilist returned non-success status".to_string(),
            context: Some(response.status().to_string()),
        });
    }

    response.json().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::AnilistMatchMissing,
        message: "anilist payload parse failed".to_string(),
        context: Some(err.to_string()),
    })
}

fn parse_catalog_items(value: serde_json::Value) -> Vec<AnimeCatalogItem> {
    value
        .get("data")
        .and_then(|v| v.get("Page"))
        .and_then(|v| v.get("media"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let anilist_id = item.get("id")?.as_i64()?;
            let title = item
                .get("title")
                .and_then(|v| v.get("english"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    item.get("title")
                        .and_then(|v| v.get("romaji"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Unknown".to_string());

            Some(AnimeCatalogItem {
                anilist_id,
                mal_id: item.get("idMal").and_then(|v| v.as_i64()),
                title,
                poster_url: item
                    .get("coverImage")
                    .and_then(|v| v.get("large"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                year: item.get("seasonYear").and_then(|v| v.as_i64()).map(|v| v as i32),
                episodes: item.get("episodes").and_then(|v| v.as_i64()).map(|v| v as i32),
            })
        })
        .collect()
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
                total_episodes: None,
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
}
