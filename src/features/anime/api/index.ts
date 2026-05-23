import { invoke } from "@tauri-apps/api/core";
import type { AnimeCommandStubResponse } from "../types";

export function animeGetDetails() {
  return invoke<AnimeCommandStubResponse>("anime_get_details");
}

export function animeGetEpisodeList() {
  return invoke<AnimeCommandStubResponse>("anime_get_episode_list");
}

export function animeResolvePlayback() {
  return invoke<AnimeCommandStubResponse>("anime_resolve_playback");
}

export function animeGetSkipTimings() {
  return invoke<AnimeCommandStubResponse>("anime_get_skip_timings");
}

export function animeSetTranslationMode() {
  return invoke<AnimeCommandStubResponse>("anime_set_translation_mode");
}

export function animePrepareDownload() {
  return invoke<AnimeCommandStubResponse>("anime_prepare_download");
}
