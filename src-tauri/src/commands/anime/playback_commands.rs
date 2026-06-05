use tauri::{AppHandle, Emitter, State};
use std::path::PathBuf;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

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
    AnimeSkipSegmentActiveEventPayload, AnimeExecuteDownloadRequest, AnimeExecuteDownloadResponse,
    AnimeCancelDownloadRequest,
    AnimeGetLocalPlaybackSourceRequest, AnimeGetLocalPlaybackSourceResponse,
    AnimeRemoveDownloadArtifactsRequest,
    EVENT_ANIME_PLAYBACK_FAILED, EVENT_ANIME_PLAYBACK_READY, EVENT_ANIME_PROGRESS_UPDATED,
    EVENT_ANIME_SKIP_SEGMENT_ACTIVE,
};

static CANCELLED_DOWNLOADS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn cancelled_downloads() -> &'static Mutex<HashSet<String>> {
    CANCELLED_DOWNLOADS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(super) fn handle_get_episode_list(
    request: AnimeGetEpisodeListRequest,
) -> Result<AnimeGetEpisodeListResponse, AnimeCommandError> {
    let mapper = AnimeMappingService::new();
    let output = mapper.map(MappingInput {
        is_movie: request.is_movie,
        tmdb_episodes: request.tmdb_episodes,
        anilist_episode_count: request.anilist_episode_count,
        released_episode_count: request.released_episode_count,
        preferred_season_number: request.preferred_season_number,
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

    let file_name = if request.source.playback_kind == crate::services::anime::models::AnimePlaybackKind::Hls
        || playback_url.contains(".m3u8")
    {
        file_name.replace(".mp4", ".ts")
    } else {
        file_name
    };

    Ok(AnimePrepareDownloadResponse {
        payload: crate::services::anime::models::AnimeDownloadPayload {
            media_name: request.request.anime_id.canonical_title.clone(),
            tmdb_id: request.request.anime_id.tmdb_id,
            season_number: request.request.season_number,
            episode_number: request.request.episode_number,
            identity_key,
            file_name,
            playback_kind: request.source.playback_kind.clone(),
            playback_url,
            referer: request.source.referer,
            request_headers,
            subtitle_candidates,
        },
    })
}

pub(super) async fn handle_execute_download(
    request: AnimeExecuteDownloadRequest,
) -> Result<AnimeExecuteDownloadResponse, AnimeCommandError> {
    clear_cancel_flag(&request.download_id);
    let output_dir = build_download_output_dir(&request.payload);
    std::fs::create_dir_all(&output_dir).map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to create download directory".to_string(),
        context: Some(err.to_string()),
    })?;

    let output_path = output_dir.join(&request.payload.file_name);
    let tmp_path = output_dir.join(format!("{}.part", request.payload.file_name));

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60 * 30))
        .build()
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "failed to initialize download client".to_string(),
            context: Some(err.to_string()),
        })?;

    let mut last_error: Option<AnimeCommandError> = None;
    for _ in 0..=2 {
        match download_to_temp_file(&client, &request.payload, &request.download_id, &tmp_path).await {
            Ok(bytes_downloaded) => {
                if is_cancelled(&request.download_id) {
                    let _ = std::fs::remove_file(&tmp_path);
                    return Err(AnimeCommandError {
                        category: AnimeErrorCategory::ProviderSearchFailed,
                        message: "download cancelled".to_string(),
                        context: None,
                    });
                }
                std::fs::rename(&tmp_path, &output_path).map_err(|err| AnimeCommandError {
                    category: AnimeErrorCategory::PlayableSourceMissing,
                    message: "failed to finalize download file".to_string(),
                    context: Some(err.to_string()),
                })?;

                return Ok(AnimeExecuteDownloadResponse {
                    output_path: output_path.display().to_string(),
                    bytes_downloaded,
                });
            }
            Err(err) => {
                let _ = std::fs::remove_file(&tmp_path);
                last_error = Some(err);
            }
        }
    }

    Err(last_error.unwrap_or(AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "download failed after retries".to_string(),
        context: None,
    }))
}

fn default_download_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join("Downloads").join("Ether");
    }
    std::env::temp_dir().join("EtherDownloads")
}

fn build_download_output_dir(payload: &crate::services::anime::models::AnimeDownloadPayload) -> PathBuf {
    build_download_output_dir_from_parts(&payload.media_name, payload.season_number)
}

fn build_download_output_dir_from_parts(media_name: &str, season_number: Option<i32>) -> PathBuf {
    let anime_name = sanitize_path_segment(media_name);
    let season = season_number
        .map(|number| format!("Season_{number:02}"))
        .unwrap_or_else(|| "Season_00".to_string());

    default_download_dir().join(anime_name).join(season)
}

fn sanitize_path_segment(input: &str) -> String {
    let cleaned = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if cleaned.is_empty() {
        "Unknown_Anime".to_string()
    } else {
        cleaned
    }
}

pub(super) fn handle_get_local_playback_source(
    request: AnimeGetLocalPlaybackSourceRequest,
) -> Result<AnimeGetLocalPlaybackSourceResponse, AnimeCommandError> {
    let media_name = request.request.anime_id.canonical_title.clone();
    let file_name = super::build_download_file_name(
        &media_name,
        request.request.season_number,
        request.request.episode_number,
    );
    let output_path = build_download_output_dir_from_parts(&media_name, request.request.season_number)
        .join(file_name);

    if !output_path.exists() {
        return Ok(AnimeGetLocalPlaybackSourceResponse { source: None });
    }

    Ok(AnimeGetLocalPlaybackSourceResponse {
        source: Some(AnimePlaybackSource {
            provider: crate::services::anime::models::AnimeProvider::Unknown,
            playback_kind: crate::services::anime::models::AnimePlaybackKind::DirectVideo,
            url: output_path.display().to_string(),
            referer: None,
            subtitle_candidates: vec![],
            is_downloadable: false,
        }),
    })
}

pub(super) fn handle_cancel_download(
    request: AnimeCancelDownloadRequest,
) -> Result<(), AnimeCommandError> {
    if let Ok(mut cancelled) = cancelled_downloads().lock() {
        cancelled.insert(request.download_id);
    }
    Ok(())
}

pub(super) fn handle_remove_download_artifacts(
    request: AnimeRemoveDownloadArtifactsRequest,
) -> Result<(), AnimeCommandError> {
    let output_dir = build_download_output_dir(&request.payload);
    let canonical_output = output_dir.join(&request.payload.file_name);
    let part_output = output_dir.join(format!("{}.part", request.payload.file_name));

    if let Some(path) = request.output_path {
        let _ = std::fs::remove_file(path);
    }
    let _ = std::fs::remove_file(canonical_output);
    let _ = std::fs::remove_file(part_output);
    prune_empty_download_dirs(&output_dir);
    Ok(())
}

fn prune_empty_download_dirs(season_dir: &PathBuf) {
    let _ = std::fs::remove_dir(season_dir);
    if let Some(anime_dir) = season_dir.parent() {
        let _ = std::fs::remove_dir(anime_dir);
    }
}

pub(crate) async fn download_to_temp_file(
    client: &reqwest::Client,
    payload: &crate::services::anime::models::AnimeDownloadPayload,
    download_id: &str,
    tmp_path: &PathBuf,
) -> Result<u64, AnimeCommandError> {
    let mut request_builder = client.get(&payload.playback_url);
    for (key, value) in &payload.request_headers {
        request_builder = request_builder.header(key, value);
    }

    let response = request_builder.send().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "failed to download anime media".to_string(),
        context: Some(err.to_string()),
    })?;
    if !response.status().is_success() {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "download request failed".to_string(),
            context: Some(format!("status={}", response.status())),
        });
    }

    let mut file = tokio::fs::File::create(tmp_path).await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to create temporary download file".to_string(),
        context: Some(err.to_string()),
    })?;

    let mut total = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if is_cancelled(download_id) {
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::ProviderSearchFailed,
                message: "download cancelled".to_string(),
                context: None,
            });
        }
        let chunk = chunk.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "failed while reading download stream".to_string(),
            context: Some(err.to_string()),
        })?;
        total += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed writing download chunk".to_string(),
            context: Some(err.to_string()),
        })?;
    }

    file.flush().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to flush download file".to_string(),
        context: Some(err.to_string()),
    })?;

    Ok(total)
}

fn is_cancelled(download_id: &str) -> bool {
    cancelled_downloads()
        .lock()
        .ok()
        .map(|set| set.contains(download_id))
        .unwrap_or(false)
}

fn clear_cancel_flag(download_id: &str) {
    if let Ok(mut set) = cancelled_downloads().lock() {
        set.remove(download_id);
    }
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
