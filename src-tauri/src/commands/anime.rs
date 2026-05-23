use serde::{Deserialize, Serialize};

pub const EVENT_ANIME_PLAYBACK_READY: &str = "anime-playback-ready";
pub const EVENT_ANIME_PLAYBACK_FAILED: &str = "anime-playback-failed";
pub const EVENT_ANIME_PROGRESS_UPDATED: &str = "anime-progress-updated";
pub const EVENT_ANIME_SKIP_SEGMENT_ACTIVE: &str = "anime-skip-segment-active";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeCommandStubResponse {
    pub command: &'static str,
    pub status: &'static str,
    pub message: &'static str,
}

#[tauri::command]
pub fn anime_get_details() -> AnimeCommandStubResponse {
    AnimeCommandStubResponse {
        command: "anime_get_details",
        status: "stub",
        message: "anime details contract scaffolded",
    }
}

#[tauri::command]
pub fn anime_get_episode_list() -> AnimeCommandStubResponse {
    AnimeCommandStubResponse {
        command: "anime_get_episode_list",
        status: "stub",
        message: "anime episode list contract scaffolded",
    }
}

#[tauri::command]
pub fn anime_resolve_playback() -> AnimeCommandStubResponse {
    AnimeCommandStubResponse {
        command: "anime_resolve_playback",
        status: "stub",
        message: "anime playback resolver contract scaffolded",
    }
}

#[tauri::command]
pub fn anime_get_skip_timings() -> AnimeCommandStubResponse {
    AnimeCommandStubResponse {
        command: "anime_get_skip_timings",
        status: "stub",
        message: "anime skip timings contract scaffolded",
    }
}

#[tauri::command]
pub fn anime_set_translation_mode() -> AnimeCommandStubResponse {
    AnimeCommandStubResponse {
        command: "anime_set_translation_mode",
        status: "stub",
        message: "anime translation mode contract scaffolded",
    }
}

#[tauri::command]
pub fn anime_prepare_download() -> AnimeCommandStubResponse {
    AnimeCommandStubResponse {
        command: "anime_prepare_download",
        status: "stub",
        message: "anime download preparation contract scaffolded",
    }
}
