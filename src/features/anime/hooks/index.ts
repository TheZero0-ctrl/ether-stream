import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  animeGetLocalPlaybackSource,
  animeGetLatest,
  animeSearch,
  animeGetDetails,
  animeGetEpisodeList,
  animeGetSkipTimings,
  resolvePlaybackSession,
} from "../api";
import type {
  AnimeDetails,
  AnimeEpisode,
  AnimeCatalogItem,
  AnimeGetDetailsRequest,
  AnimeGetEpisodeListRequest,
  AnimeGetSkipTimingsRequest,
  AnimePlaybackRequest,
  AnimePlaybackSourceOption,
  AnimeSkipTimings,
  AnimeTranslationMode,
} from "../types";

function extractInvokeError(err: unknown, fallback: string): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (typeof err === "object" && err !== null) {
    const candidate = err as { message?: unknown; category?: unknown; context?: unknown };
    if (typeof candidate.message === "string") {
      const suffix = typeof candidate.context === "string" && candidate.context
        ? ` (${candidate.context})`
        : "";
      return `${candidate.message}${suffix}`;
    }
    try {
      return JSON.stringify(err);
    } catch {
      return fallback;
    }
  }
  return fallback;
}

export function useAnimeCatalog() {
  const [latestItems, setLatestItems] = useState<AnimeCatalogItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshLatest = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await animeGetLatest({ limit: 20 });
      setLatestItems(result.items);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Failed to load latest anime");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      setLoading(true);
      setError(null);
      try {
        const result = await animeGetLatest({ limit: 20 });
        if (!cancelled) setLatestItems(result.items);
      } catch (err: unknown) {
        if (!cancelled) setError(err instanceof Error ? err.message : "Failed to load latest anime");
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const search = async (query: string) => {
    try {
      await animeSearch({ query, limit: 20 });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Search failed");
    }
  };

  return { latestItems, loading, error, search, refreshLatest };
}

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
  const [sources, setSources] = useState<AnimePlaybackSourceOption[]>([]);
  const [selectedSourceId, setSelectedSourceId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const inFlightKey = useRef<string | null>(null);
  const requestVersion = useRef(0);

  const request = useMemo(() => {
    if (!baseRequest) return null;
    return { ...baseRequest, translationMode };
  }, [baseRequest, translationMode]);

  const source = useMemo(
    () => sources.find((item) => item.id === selectedSourceId)?.source ?? sources[0]?.source ?? null,
    [sources, selectedSourceId],
  );

  const addSource = useCallback((option: AnimePlaybackSourceOption) => {
    setSources((current) => {
      const duplicate = current.some((item) => item.source.url === option.source.url);
      if (duplicate) return current;
      return [...current, option];
    });
  }, []);

  const resolve = useCallback(async () => {
    if (!request) return;
    const key = JSON.stringify({
      id: request.animeId,
      season: request.seasonNumber,
      episode: request.episodeNumber,
      mode: translationMode,
    });
    if (inFlightKey.current === key) return;
    inFlightKey.current = key;
    const version = ++requestVersion.current;
    setLoading(true);
    setError(null);
    try {
      const result = await resolvePlaybackSession(request);
      if (requestVersion.current === version) {
        const primaryId = `primary:${result.source.provider}:${result.source.playbackKind}`;
        setSources([
          {
            id: primaryId,
            label: `${result.source.provider} (${translationMode.toUpperCase()})`,
            source: result.source,
            origin: "primary",
          },
        ]);
        setSelectedSourceId(primaryId);

        const altMode: AnimeTranslationMode = translationMode === "sub" ? "dub" : "sub";
        const backgroundRequest = { ...request, translationMode: altMode };

        void resolvePlaybackSession(backgroundRequest)
          .then((fallback) => {
            if (requestVersion.current !== version) return;
            addSource({
              id: `background:${fallback.source.provider}:${altMode}`,
              label: `${fallback.source.provider} (${altMode.toUpperCase()})`,
              source: fallback.source,
              origin: "background",
            });
          })
          .catch(() => {});

        void animeGetLocalPlaybackSource({ request })
          .then((local) => {
            if (requestVersion.current !== version || !local.source) return;
            addSource({
              id: "local:file",
              label: "Local File",
              source: local.source,
              origin: "local",
            });
          })
          .catch(() => {});
      }
    } catch (err: unknown) {
      if (requestVersion.current === version) {
        setError(extractInvokeError(err, "Playback resolution failed"));
      }
    } finally {
      inFlightKey.current = null;
      if (requestVersion.current === version) {
        setLoading(false);
      }
    }
  }, [request, translationMode]);

  useEffect(() => {
    if (!request) return;
    const timer = window.setTimeout(() => {
      void resolve();
    }, 200);
    return () => {
      window.clearTimeout(timer);
    };
  }, [request, resolve]);

  return {
    translationMode,
    setTranslationMode,
    source,
    sources,
    selectedSourceId,
    setSelectedSourceId,
    error,
    loading,
    resolve,
  };
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
