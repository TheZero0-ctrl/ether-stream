use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::models::{AnimeIdentity, AnimePlaybackKind, AnimePlaybackSource, AnimeTranslationMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackRuntimeMode {
    RemoteWebview,
    LocalProxy,
    OfflineFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimePlaybackSession {
    pub session_id: String,
    pub identity: AnimeIdentity,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub translation_mode: AnimeTranslationMode,
    pub runtime_mode: PlaybackRuntimeMode,
    pub source: AnimePlaybackSource,
    pub resume_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub watched_completed: bool,
    pub ended_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressUpdate {
    pub progress_seconds: f64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct AnimePlaybackService;

impl AnimePlaybackService {
    pub fn new() -> Self {
        Self
    }

    pub fn start_session(
        &self,
        identity: AnimeIdentity,
        season_number: Option<i32>,
        episode_number: Option<i32>,
        translation_mode: AnimeTranslationMode,
        source: AnimePlaybackSource,
        resume_seconds: Option<u64>,
    ) -> AnimePlaybackSession {
        let runtime_mode = select_runtime_mode(&source.playback_kind);
        let session_id = build_session_id(identity.tmdb_id, season_number, episode_number);

        AnimePlaybackSession {
            session_id,
            identity,
            season_number,
            episode_number,
            translation_mode,
            runtime_mode,
            source,
            resume_seconds: resume_seconds.unwrap_or(0) as f64,
            duration_seconds: None,
            watched_completed: false,
            ended_at_unix_ms: None,
        }
    }

    pub fn update_progress(
        &self,
        session: &AnimePlaybackSession,
        update: ProgressUpdate,
    ) -> AnimePlaybackSession {
        let watched_completed = watched_completed(update.progress_seconds, update.duration_seconds);

        AnimePlaybackSession {
            duration_seconds: update.duration_seconds,
            resume_seconds: update.progress_seconds,
            watched_completed,
            ..session.clone()
        }
    }

    pub fn end_session(
        &self,
        session: &AnimePlaybackSession,
        final_progress: Option<ProgressUpdate>,
    ) -> AnimePlaybackSession {
        let progress_seconds = final_progress
            .as_ref()
            .map(|value| value.progress_seconds)
            .unwrap_or(session.resume_seconds);
        let duration_seconds = final_progress
            .as_ref()
            .and_then(|value| value.duration_seconds)
            .or(session.duration_seconds);

        AnimePlaybackSession {
            resume_seconds: progress_seconds,
            duration_seconds,
            watched_completed: watched_completed(progress_seconds, duration_seconds),
            ended_at_unix_ms: Some(now_unix_ms()),
            ..session.clone()
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn watched_completed(progress_seconds: f64, duration_seconds: Option<f64>) -> bool {
    if let Some(duration) = duration_seconds {
        if duration <= 0.0 {
            return false;
        }
        let ratio = progress_seconds / duration;
        return ratio >= 0.9;
    }
    false
}

pub fn build_identity_key(identity: &AnimeIdentity) -> String {
    format!(
        "tmdb:{:?}|anilist:{:?}|mal:{:?}",
        identity.tmdb_id, identity.anilist_id, identity.mal_id
    )
}

pub fn build_episode_progress_key(
    identity: &AnimeIdentity,
    season_number: Option<i32>,
    episode_number: Option<i32>,
) -> String {
    format!(
        "tmdb:{:?}|anilist:{:?}|mal:{:?}|season:{:?}|episode:{:?}",
        identity.tmdb_id, identity.anilist_id, identity.mal_id, season_number, episode_number
    )
}

fn select_runtime_mode(kind: &AnimePlaybackKind) -> PlaybackRuntimeMode {
    match kind {
        AnimePlaybackKind::WebviewRemote => PlaybackRuntimeMode::RemoteWebview,
        AnimePlaybackKind::LocalProxy => PlaybackRuntimeMode::LocalProxy,
        AnimePlaybackKind::DirectVideo | AnimePlaybackKind::Hls => PlaybackRuntimeMode::OfflineFile,
    }
}

fn build_session_id(tmdb_id: Option<i64>, season_number: Option<i32>, episode_number: Option<i32>) -> String {
    format!(
        "anime-session:{}:{}:{}",
        tmdb_id.map(|value| value.to_string()).unwrap_or_else(|| "na".to_string()),
        season_number.map(|value| value.to_string()).unwrap_or_else(|| "na".to_string()),
        episode_number
            .map(|value| value.to_string())
            .unwrap_or_else(|| "na".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::anime::models::{
        AnimeMediaKind, AnimePlaybackKind, AnimeProvider, DiscoveredSubtitle,
    };

    fn identity() -> AnimeIdentity {
        AnimeIdentity {
            media_kind: AnimeMediaKind::AnimeSeries,
            tmdb_id: Some(7),
            anilist_id: Some(8),
            mal_id: Some(9),
            canonical_title: "Test".to_string(),
            romaji_title: None,
            english_title: None,
            native_title: None,
            title_aliases: vec![],
        }
    }

    fn source(kind: AnimePlaybackKind) -> AnimePlaybackSource {
        AnimePlaybackSource {
            provider: AnimeProvider::Gogoanime,
            playback_kind: kind,
            url: "https://stream".to_string(),
            referer: None,
            subtitle_candidates: vec![DiscoveredSubtitle {
                language: "en".to_string(),
                label: Some("English".to_string()),
                url: None,
            }],
            is_downloadable: true,
        }
    }

    #[test]
    fn starts_session_with_resume_value() {
        let service = AnimePlaybackService::new();
        let session = service.start_session(
            identity(),
            Some(1),
            Some(2),
            AnimeTranslationMode::Sub,
            source(AnimePlaybackKind::WebviewRemote),
            Some(120),
        );

        assert_eq!(session.resume_seconds, 120.0);
        assert!(matches!(session.runtime_mode, PlaybackRuntimeMode::RemoteWebview));
    }

    #[test]
    fn updates_progress_and_marks_watched_threshold() {
        let service = AnimePlaybackService::new();
        let session = service.start_session(
            identity(),
            Some(1),
            Some(2),
            AnimeTranslationMode::Sub,
            source(AnimePlaybackKind::WebviewRemote),
            None,
        );

        let updated = service.update_progress(
            &session,
            ProgressUpdate {
                progress_seconds: 540.0,
                duration_seconds: Some(600.0),
            },
        );

        assert!(updated.watched_completed);
    }

    #[test]
    fn switches_mode_based_on_playback_kind() {
        let service = AnimePlaybackService::new();
        let remote = service.start_session(
            identity(),
            Some(1),
            Some(1),
            AnimeTranslationMode::Sub,
            source(AnimePlaybackKind::WebviewRemote),
            None,
        );
        let local = service.start_session(
            identity(),
            Some(1),
            Some(1),
            AnimeTranslationMode::Sub,
            source(AnimePlaybackKind::LocalProxy),
            None,
        );

        assert!(matches!(remote.runtime_mode, PlaybackRuntimeMode::RemoteWebview));
        assert!(matches!(local.runtime_mode, PlaybackRuntimeMode::LocalProxy));
    }

    #[test]
    fn builds_stable_episode_progress_key() {
        let key = build_episode_progress_key(&identity(), Some(1), Some(7));
        assert!(key.contains("season:Some(1)"));
        assert!(key.contains("episode:Some(7)"));
    }

    #[test]
    fn ends_session_with_timestamp_and_final_progress() {
        let service = AnimePlaybackService::new();
        let session = service.start_session(
            identity(),
            Some(1),
            Some(2),
            AnimeTranslationMode::Sub,
            source(AnimePlaybackKind::WebviewRemote),
            Some(10),
        );

        let ended = service.end_session(
            &session,
            Some(ProgressUpdate {
                progress_seconds: 580.0,
                duration_seconds: Some(600.0),
            }),
        );

        assert_eq!(ended.resume_seconds, 580.0);
        assert!(ended.watched_completed);
        assert!(ended.ended_at_unix_ms.is_some());
    }
}
