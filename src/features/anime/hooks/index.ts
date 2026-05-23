import { useEffect, useMemo, useState } from "react";
import {
  animeGetDetails,
  animeGetEpisodeList,
  animeGetSkipTimings,
  animeResolvePlayback,
  animeSetTranslationMode,
} from "../api";
import type {
  AnimeDetails,
  AnimeEpisode,
  AnimeGetDetailsRequest,
  AnimeGetEpisodeListRequest,
  AnimeGetSkipTimingsRequest,
  AnimePlaybackRequest,
  AnimePlaybackSource,
  AnimeSkipTimings,
  AnimeTranslationMode,
} from "../types";

export function useAnimeDetails(request: AnimeGetDetailsRequest) {
  const [details, setDetails] = useState<AnimeDetails | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    animeGetDetails(request)
      .then((result) => {
        if (!cancelled) setDetails(result.details);
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load anime details");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [request]);

  return { details, loading, error };
}

export function useAnimeEpisodes(request: AnimeGetEpisodeListRequest | null) {
  const [episodes, setEpisodes] = useState<AnimeEpisode[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!request) return;
    let cancelled = false;
    setLoading(true);
    setError(null);

    animeGetEpisodeList(request)
      .then((result) => {
        if (!cancelled) setEpisodes(result.seasons.flatMap((season) => season.episodes));
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load episodes");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [request]);

  return { episodes, loading, error };
}

export function useAnimePlaybackSession(baseRequest: AnimePlaybackRequest | null) {
  const [translationMode, setTranslationMode] = useState<AnimeTranslationMode>("sub");
  const [source, setSource] = useState<AnimePlaybackSource | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const request = useMemo(() => {
    if (!baseRequest) return null;
    return { ...baseRequest, translationMode };
  }, [baseRequest, translationMode]);

  const resolve = async () => {
    if (!request) return;
    setLoading(true);
    setError(null);
    try {
      await animeSetTranslationMode({
        identity: request.animeId,
        translationMode,
      });
      const result = await animeResolvePlayback(request);
      setSource(result.source);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Playback resolution failed");
    } finally {
      setLoading(false);
    }
  };

  return { translationMode, setTranslationMode, source, error, loading, resolve };
}

export function useAnimeSkipState(request: AnimeGetSkipTimingsRequest | null) {
  const [timings, setTimings] = useState<AnimeSkipTimings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!request) return;
    animeGetSkipTimings(request)
      .then((result) => setTimings(result.timings))
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : "Failed to load skip timings");
      });
  }, [request]);

  return { timings, error };
}
