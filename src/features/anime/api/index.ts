import { invoke } from "@tauri-apps/api/core";
import type {
  AnimeGetDetailsRequest,
  AnimeGetDetailsResponse,
  AnimeGetEpisodeListRequest,
  AnimeGetEpisodeListResponse,
  AnimeGetSkipTimingsRequest,
  AnimeGetSkipTimingsResponse,
  AnimePlaybackRequest,
  AnimePrepareDownloadRequest,
  AnimePrepareDownloadResponse,
  AnimeResolvePlaybackResponse,
  AnimeSetTranslationModeRequest,
  AnimeSetTranslationModeResponse,
} from "../types";

export function animeGetDetails(request: AnimeGetDetailsRequest) {
  return invoke<AnimeGetDetailsResponse>("anime_get_details", { request });
}

export function animeGetEpisodeList(request: AnimeGetEpisodeListRequest) {
  return invoke<AnimeGetEpisodeListResponse>("anime_get_episode_list", { request });
}

export function animeResolvePlayback(request: AnimePlaybackRequest) {
  return invoke<AnimeResolvePlaybackResponse>("anime_resolve_playback", { request });
}

export function animeGetSkipTimings(request: AnimeGetSkipTimingsRequest) {
  return invoke<AnimeGetSkipTimingsResponse>("anime_get_skip_timings", { request });
}

export function animeSetTranslationMode(request: AnimeSetTranslationModeRequest) {
  return invoke<AnimeSetTranslationModeResponse>("anime_set_translation_mode", { request });
}

export function animePrepareDownload(request: AnimePrepareDownloadRequest) {
  return invoke<AnimePrepareDownloadResponse>("anime_prepare_download", { request });
}
