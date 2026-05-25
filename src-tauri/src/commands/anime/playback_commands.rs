use tauri::{AppHandle, Emitter, State};

use crate::database::AppDatabase;
use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::mapping::{AnimeMappingService, MappingInput};
use crate::services::anime::models::{
    AnimeIdentity, AnimeIntroSkipMode, AnimeMediaKind, AnimePlaybackSource, AnimeSettings,
    AnimeSkipTimings,
};
use crate::services::anime::playback::{
    build_episode_progress_key, build_identity_key, AnimePlaybackService, ProgressUpdate,
};
use crate::services::anime::repository_sqlx::{AnimeSqlxCacheRepository, AnimeSqlxProgressRepository};
use crate::services::anime::resolver::{AnimeResolverService, ResolverContext};
use crate::services::anime::skip::{AnimeSkipMode, AnimeSkipService};

use super::{
    AnimeGetEpisodeListRequest, AnimeGetEpisodeListResponse, AnimeGetSkipTimingsRequest,
    AnimeGetSkipTimingsResponse, AnimePlaybackFailedEventPayload, AnimePlaybackReadyEventPayload,
    AnimePrepareDownloadRequest, AnimePrepareDownloadResponse, AnimeProgressUpdatedEventPayload,
    AnimeResolvePlaybackResponse, AnimeSetTranslationModeRequest, AnimeSetTranslationModeResponse,
    AnimeSkipSegmentActiveEventPayload, EVENT_ANIME_PLAYBACK_FAILED, EVENT_ANIME_PLAYBACK_READY,
    EVENT_ANIME_PROGRESS_UPDATED, EVENT_ANIME_SKIP_SEGMENT_ACTIVE,
};

pub(super) fn handle_get_episode_list(
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

pub(super) async fn handle_resolve_playback(
    request: crate::services::anime::models::AnimePlaybackRequest,
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

pub(super) async fn handle_get_skip_timings(
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

pub(super) fn handle_set_translation_mode(
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

pub(super) fn handle_prepare_download(
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
    let file_name = super::build_download_file_name(
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
        payload: crate::services::anime::models::AnimeDownloadPayload {
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
