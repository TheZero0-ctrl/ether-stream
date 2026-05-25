use tauri::State;

use crate::database::AppDatabase;
use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::playback::{build_episode_progress_key, build_identity_key, watched_completed};
use crate::services::anime::repository_sqlx::AnimeSqlxProgressRepository;

use super::{
    parse_episode_progress_key, AnimeResumeProgressRequest, AnimeResumeProgressResponse,
    AnimeSetLastEpisodeRequest, AnimeUpdateProgressRequest,
};

pub(super) async fn handle_get_resume_progress(
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

pub(super) async fn handle_update_progress(
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

    let watched_done = watched_completed(request.progress_seconds, request.duration_seconds);

    repository
        .upsert_progress(
            &episode_key,
            &identity_key,
            request.progress_seconds,
            request.duration_seconds,
            watched_done,
        )
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to persist anime playback progress".to_string(),
            context: Some(err.to_string()),
        })
}

pub(super) async fn handle_set_last_episode(
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
