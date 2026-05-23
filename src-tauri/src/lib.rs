use serde::Serialize;

mod commands;
mod services;

use commands::anime::{
    anime_get_details, anime_get_episode_list, anime_get_skip_timings, anime_prepare_download,
    anime_resolve_playback, anime_set_translation_mode,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    name: &'static str,
    version: &'static str,
    frontend: &'static str,
    backend: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapState {
    app: AppMetadata,
    capabilities: Vec<&'static str>,
    next_steps: Vec<&'static str>,
}

#[tauri::command]
fn get_bootstrap_state() -> BootstrapState {
    BootstrapState {
        app: AppMetadata {
            name: "Ether",
            version: env!("CARGO_PKG_VERSION"),
            frontend: "React + TypeScript",
            backend: "Rust + Tauri",
        },
        capabilities: vec![
            "native secrets and settings",
            "desktop filesystem access",
            "downloads orchestration",
            "subtitle and source services",
            "window and updater integration",
        ],
        next_steps: vec![
            "add persistent storage layer",
            "define TMDB and AniList client modules",
            "port download manager into Rust services",
            "build media library and playback flows",
        ],
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_state,
            anime_get_details,
            anime_get_episode_list,
            anime_resolve_playback,
            anime_get_skip_timings,
            anime_set_translation_mode,
            anime_prepare_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
