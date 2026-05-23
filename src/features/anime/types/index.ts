export type AnimeCommandStubResponse = {
  command: string;
  status: "stub";
  message: string;
};

export const ANIME_EVENTS = {
  playbackReady: "anime-playback-ready",
  playbackFailed: "anime-playback-failed",
  progressUpdated: "anime-progress-updated",
  skipSegmentActive: "anime-skip-segment-active",
} as const;

export type AnimePlaybackReadyPayload = {
  status: "stub";
  message: string;
};

export type AnimePlaybackFailedPayload = {
  code: string;
  message: string;
};

export type AnimeProgressUpdatedPayload = {
  progressSeconds: number;
  durationSeconds: number;
};

export type AnimeSkipSegmentActivePayload = {
  segmentType: "intro" | "outro";
  startSeconds: number;
  endSeconds: number;
};
