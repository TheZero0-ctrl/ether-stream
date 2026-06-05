import { invoke } from "@tauri-apps/api/core";
import type {
  AnimeGetDetailsRequest,
  AnimeGetDetailsResponse,
  AnimeGetEpisodeListRequest,
  AnimeGetEpisodeListResponse,
  AnimeLatestRequest,
  AnimeLatestResponse,
  AnimeResumeProgressRequest,
  AnimeResumeProgressResponse,
  AnimeSearchRequest,
  AnimeSearchResponse,
  AnimeGetSkipTimingsRequest,
  AnimeGetSkipTimingsResponse,
  AnimePlaybackRequest,
  AnimePrepareDownloadRequest,
  AnimePrepareDownloadResponse,
  AnimeExecuteDownloadRequest,
  AnimeExecuteDownloadResponse,
  AnimeCancelDownloadRequest,
  AnimeDownloadsEnqueueRequest,
  AnimeDownloadsListResponse,
  AnimeRemoveDownloadArtifactsRequest,
  AnimeGetLocalPlaybackSourceRequest,
  AnimeGetLocalPlaybackSourceResponse,
  AnimeResolvePlaybackResponse,
  AnimeSetTranslationModeRequest,
  AnimeSetTranslationModeResponse,
  AnimeSetLastEpisodeRequest,
  AnimeUpdateProgressRequest,
} from "../types";

export function animeGetDetails(request: AnimeGetDetailsRequest) {
  return invoke<AnimeGetDetailsResponse>("anime_get_details", { request });
}

export function animeGetLatest(request: AnimeLatestRequest) {
  return invoke<AnimeLatestResponse>("anime_get_latest", { request });
}

export function animeSearch(request: AnimeSearchRequest) {
  return invoke<AnimeSearchResponse>("anime_search", { request });
}

export function animeGetResumeProgress(request: AnimeResumeProgressRequest) {
  return invoke<AnimeResumeProgressResponse | null>("anime_get_resume_progress", { request });
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

export function animeExecuteDownload(request: AnimeExecuteDownloadRequest) {
  return invoke<AnimeExecuteDownloadResponse>("anime_execute_download", { request });
}

export function animeCancelDownload(request: AnimeCancelDownloadRequest) {
  return invoke<void>("anime_cancel_download", { request });
}

export function animeDownloadsEnqueue(request: AnimeDownloadsEnqueueRequest) {
  return invoke<void>("anime_downloads_enqueue", { request });
}

export function animeDownloadsList() {
  return invoke<AnimeDownloadsListResponse>("anime_downloads_list");
}

export function animeDownloadsCancel(request: AnimeCancelDownloadRequest) {
  return invoke<void>("anime_downloads_cancel", { request });
}

export function animeDownloadsRemove(request: AnimeCancelDownloadRequest) {
  return invoke<void>("anime_downloads_remove", { request });
}

export function animeRemoveDownloadArtifacts(request: AnimeRemoveDownloadArtifactsRequest) {
  return invoke<void>("anime_remove_download_artifacts", { request });
}

export function animeGetLocalPlaybackSource(request: AnimeGetLocalPlaybackSourceRequest) {
  return invoke<AnimeGetLocalPlaybackSourceResponse>("anime_get_local_playback_source", { request });
}

export function animeUpdateProgress(request: AnimeUpdateProgressRequest) {
  return invoke<void>("anime_update_progress", { request });
}

export function animeSetLastEpisode(request: AnimeSetLastEpisodeRequest) {
  return invoke<void>("anime_set_last_episode", { request });
}

export async function resolvePlaybackSession(request: AnimePlaybackRequest) {
  await animeSetTranslationMode({
    identity: request.animeId,
    translationMode: request.translationMode,
  });

  const resume = await animeGetResumeProgress({
    identity: request.animeId,
    seasonNumber: request.seasonNumber,
    episodeNumber: request.episodeNumber,
  });

  const requestWithResume = {
    ...request,
    resumeSeconds: resume?.progressSeconds ?? request.resumeSeconds,
  };

  return animeResolvePlayback(requestWithResume);
}
