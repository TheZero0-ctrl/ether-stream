export type AnimeMediaKind = "animeSeries" | "animeMovie";

export type AnimeIdentity = {
  mediaKind: AnimeMediaKind;
  tmdbId: number | null;
  anilistId: number | null;
  malId: number | null;
  canonicalTitle: string;
  romajiTitle: string | null;
  englishTitle: string | null;
  nativeTitle: string | null;
  titleAliases: string[];
};

export type AnimeSourceConfidence = "high" | "medium" | "low" | "unknown";

export type AnimeDetails = {
  identity: AnimeIdentity;
  overview: string | null;
  posterUrl: string | null;
  backdropUrl: string | null;
  genres: string[];
  score: number | null;
  releaseYear: number | null;
  status: string | null;
  sourceConfidence: AnimeSourceConfidence;
};

export type AnimeSeasonSourceKind =
  | "tmdb"
  | "anilist"
  | "hybrid"
  | "providerDerived";

export type AnimeProvider = "gogoanime" | "zoro" | "animePahe" | "unknown";

export type TmdbEpisodeRef = {
  seasonNumber: number;
  episodeNumber: number;
};

export type ProviderEpisodeRef = {
  provider: AnimeProvider;
  providerShowId: string | null;
  providerEpisodeToken: string | null;
  providerSeasonNumber: number | null;
  providerEpisodeNumber: string | null;
};

export type AnimeEpisode = {
  displayEpisodeNumber: number;
  canonicalEpisodeNumber: number;
  seasonNumber: number;
  title: string | null;
  overview: string | null;
  runtimeMinutes: number | null;
  airDate: string | null;
  tmdbReference: TmdbEpisodeRef | null;
  providerReference: ProviderEpisodeRef | null;
};

export type AnimeSeason = {
  seasonNumber: number;
  title: string;
  episodeCount: number | null;
  sourceKind: AnimeSeasonSourceKind;
  episodes: AnimeEpisode[];
};

export type AnimeTranslationMode = "sub" | "dub";

export type AnimePlaybackRequest = {
  animeId: AnimeIdentity;
  translationMode: AnimeTranslationMode;
  movie: boolean;
  seasonNumber: number | null;
  episodeNumber: number | null;
  resumeSeconds: number | null;
};

export type AnimePlaybackKind =
  | "webviewRemote"
  | "localProxy"
  | "directVideo"
  | "hls";

export type DiscoveredSubtitle = {
  language: string;
  label: string | null;
  url: string | null;
};

export type AnimePlaybackSource = {
  provider: AnimeProvider;
  playbackKind: AnimePlaybackKind;
  url: string;
  referer: string | null;
  subtitleCandidates: DiscoveredSubtitle[];
  isDownloadable: boolean;
};

export type SkipSegmentKind = "intro" | "outro";

export type SkipSegment = {
  kind: SkipSegmentKind;
  startSeconds: number;
  endSeconds: number;
};

export type AnimeSkipTimings = {
  malId: number;
  episodeNumber: number;
  segments: SkipSegment[];
};

export type ResolvedSubtitle = {
  language: string;
  label: string | null;
  filePath: string | null;
  url: string | null;
};

export type AnimeDownloadPayload = {
  mediaName: string;
  tmdbId: number | null;
  seasonNumber: number | null;
  episodeNumber: number | null;
  playbackUrl: string;
  referer: string | null;
  subtitleCandidates: ResolvedSubtitle[];
};

export type AnimeIntroSkipMode = "off" | "auto" | "manual";

export type AnimeSettings = {
  defaultTranslationMode: AnimeTranslationMode;
  introSkipMode: AnimeIntroSkipMode;
  preferredSubtitleLanguage: string;
};

export type AnimeErrorCategory =
  | "animeNotClassified"
  | "anilistMatchMissing"
  | "seasonMappingFailed"
  | "providerSearchFailed"
  | "providerEpisodeMissing"
  | "playableSourceMissing"
  | "translationUnavailable";

export type AnimeCommandError = {
  category: AnimeErrorCategory;
  message: string;
  context: string | null;
};

export type AnimeGetDetailsRequest = {
  tmdbId: number | null;
  anilistId: number | null;
  malId: number | null;
  title: string | null;
  overview: string | null;
  posterUrl: string | null;
  backdropUrl: string | null;
  genres: string[] | null;
  releaseYear: number | null;
  status: string | null;
  hasAnimationGenre: boolean;
  originalLanguage: string | null;
  originCountries: string[];
};

export type AnimeGetDetailsResponse = {
  details: AnimeDetails;
};

export type AnimeGetEpisodeListRequest = {
  identity: AnimeIdentity;
  isMovie: boolean;
  tmdbEpisodes: MappingEpisodeInput[];
  anilistEpisodeCount: number | null;
};

export type AnimeGetEpisodeListResponse = {
  identity: AnimeIdentity;
  seasons: AnimeSeason[];
};

export type MappingEpisodeInput = {
  tmdbSeasonNumber: number | null;
  tmdbEpisodeNumber: number | null;
  anilistEpisodeNumber: number | null;
  title: string | null;
  runtimeMinutes: number | null;
};

export type AnimeResolvePlaybackResponse = {
  request: AnimePlaybackRequest;
  source: AnimePlaybackSource;
};

export type AnimeGetSkipTimingsRequest = {
  malId: number;
  episodeNumber: number;
};

export type AnimeGetSkipTimingsResponse = {
  timings: AnimeSkipTimings;
};

export type AnimeSetTranslationModeRequest = {
  identity: AnimeIdentity;
  translationMode: AnimeTranslationMode;
};

export type AnimeSetTranslationModeResponse = {
  settings: AnimeSettings;
};

export type AnimePrepareDownloadRequest = {
  request: AnimePlaybackRequest;
  source: AnimePlaybackSource;
};

export type AnimePrepareDownloadResponse = {
  payload: AnimeDownloadPayload;
};

export const ANIME_EVENTS = {
  playbackReady: "anime-playback-ready",
  playbackFailed: "anime-playback-failed",
  progressUpdated: "anime-progress-updated",
  skipSegmentActive: "anime-skip-segment-active",
} as const;

export type AnimePlaybackReadyPayload = {
  identity: AnimeIdentity;
  source: AnimePlaybackSource;
  translationMode: AnimeTranslationMode;
};

export type AnimePlaybackFailedPayload = {
  error: AnimeCommandError;
};

export type AnimeProgressUpdatedPayload = {
  identity: AnimeIdentity;
  seasonNumber: number | null;
  episodeNumber: number | null;
  progressSeconds: number;
  durationSeconds: number;
};

export type AnimeSkipSegmentActivePayload = {
  identity: AnimeIdentity;
  segment: SkipSegment;
};
