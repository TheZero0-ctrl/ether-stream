import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AnimeBadge,
  EpisodeSelector,
  PlaybackErrorState,
  SkipPrompt,
  TranslationToggle,
} from "./features/anime/components";
import {
  useAnimeDetails,
  useAnimeCatalog,
  useAnimeEpisodes,
  useAnimePlaybackSession,
  useAnimeSkipState,
} from "./features/anime/hooks";
import {
  animeDownloadsCancel,
  animeDownloadsEnqueue,
  animeDownloadsList,
  animeDownloadsRemove,
  animeGetResumeProgress,
  animePrepareDownload,
  animeSetLastEpisode,
  animeUpdateProgress,
} from "./features/anime/api";
import type { AnimeDownloadRecord, AnimeIdentity } from "./features/anime/types";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import "./App.css";

function buildIdentityKey(identity: { tmdbId: number | null; anilistId: number | null; malId: number | null }) {
  return `tmdb:${identity.tmdbId ?? "none"}|anilist:${identity.anilistId ?? "none"}|mal:${identity.malId ?? "none"}`;
}

function parseSeasonHint(title: string | null | undefined): number | null {
  if (!title) return null;
  const lower = title.toLowerCase();
  const digitMatch = lower.match(/(\d+)(st|nd|rd|th)?\s+season/);
  if (digitMatch) return Number(digitMatch[1]);
  const wordMap: Record<string, number> = {
    first: 1,
    second: 2,
    third: 3,
    fourth: 4,
    fifth: 5,
    sixth: 6,
    seventh: 7,
    eighth: 8,
    ninth: 9,
    tenth: 10,
  };
  for (const [word, number] of Object.entries(wordMap)) {
    if (lower.includes(`${word} season`)) return number;
  }
  return null;
}

function PlaybackSurface({
  source,
  identity,
  seasonNumber,
  episodeNumber,
  resumeSeconds,
}: {
  source: { url: string; playbackKind: string };
  identity: AnimeIdentity;
  seasonNumber: number | null;
  episodeNumber: number | null;
  resumeSeconds: number;
}) {
  const mediaUrl = useMemo(() => {
    const isAbsoluteLocalPath = source.url.startsWith("/") || /^[A-Za-z]:\\/.test(source.url);
    if (isAbsoluteLocalPath) {
      return convertFileSrc(source.url);
    }
    return source.url;
  }, [source.url]);
  const isHls = source.playbackKind === "hls" || mediaUrl.includes(".m3u8");
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const [hlsFailed, setHlsFailed] = useState(false);

  useEffect(() => {
    setHlsFailed(false);
  }, [source.url, source.playbackKind]);

  useEffect(() => {
    if (!isHls || hlsFailed) return;
    const video = videoRef.current;
    if (!video) return;

    let active = true;
    let hlsInstance: { destroy: () => void } | null = null;

    const attach = async () => {
      try {
        const mod = await import("hls.js");
        const HlsCtor = mod.default;

        if (!active) return;

        if (video.canPlayType("application/vnd.apple.mpegurl")) {
          video.src = mediaUrl;
          void video.play().catch(() => {});
          return;
        }

        if (HlsCtor.isSupported()) {
          const hls = new HlsCtor({
            enableWorker: true,
            lowLatencyMode: true,
            maxBufferLength: 20,
          });
          hlsInstance = hls;
          hls.loadSource(mediaUrl);
          hls.attachMedia(video);
          hls.on(HlsCtor.Events.MANIFEST_PARSED, () => {
            void video.play().catch(() => {});
          });
          hls.on(HlsCtor.Events.ERROR, (_event, data) => {
            if (data?.fatal) {
              setHlsFailed(true);
            }
          });
          return;
        }

        setHlsFailed(true);
      } catch {
        if (active) setHlsFailed(true);
      }
    };

    void attach();

    return () => {
      active = false;
      if (hlsInstance) hlsInstance.destroy();
    };
  }, [isHls, mediaUrl, hlsFailed]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    const applyResumePosition = () => {
      if (!Number.isFinite(resumeSeconds) || resumeSeconds <= 0) return;
      if (video.currentTime <= 0.5) {
        video.currentTime = resumeSeconds;
      }
    };

    video.addEventListener("loadedmetadata", applyResumePosition);
    applyResumePosition();

    return () => {
      video.removeEventListener("loadedmetadata", applyResumePosition);
    };
  }, [mediaUrl, resumeSeconds]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    let lastSavedAt = 0;
    const persist = () => {
      const now = Date.now();
      if (now - lastSavedAt < 5000) return;
      const progressSeconds = video.currentTime;
      if (!Number.isFinite(progressSeconds) || progressSeconds <= 0) return;
      lastSavedAt = now;
      void animeUpdateProgress({
        identity,
        seasonNumber,
        episodeNumber,
        progressSeconds,
        durationSeconds: Number.isFinite(video.duration) ? video.duration : null,
      });
    };

    const persistFinal = () => {
      const progressSeconds = video.currentTime;
      if (!Number.isFinite(progressSeconds) || progressSeconds <= 0) return;
      void animeUpdateProgress({
        identity,
        seasonNumber,
        episodeNumber,
        progressSeconds,
        durationSeconds: Number.isFinite(video.duration) ? video.duration : null,
      });
    };

    video.addEventListener("timeupdate", persist);
    video.addEventListener("pause", persistFinal);
    video.addEventListener("ended", persistFinal);

    return () => {
      video.removeEventListener("timeupdate", persist);
      video.removeEventListener("pause", persistFinal);
      video.removeEventListener("ended", persistFinal);
    };
  }, [identity, seasonNumber, episodeNumber, mediaUrl]);

  if (source.playbackKind === "webviewRemote" && !isHls) {
    return (
      <iframe
        className="player-frame"
        src={mediaUrl}
        title="Anime playback"
        allow="autoplay; fullscreen; picture-in-picture"
      />
    );
  }

  if (hlsFailed) {
    return (
      <iframe
        className="player-frame"
        src={mediaUrl}
        title="Anime playback fallback"
        allow="autoplay; fullscreen; picture-in-picture"
      />
    );
  }

  return (
    <video ref={videoRef} className="player-video" controls autoPlay playsInline src={isHls ? undefined : mediaUrl}>
      {isHls ? "Your runtime does not support this HLS stream." : "Your runtime does not support video playback."}
    </video>
  );
}

async function windowControl(action: "close") {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    if (action === "close") await win.close();
  } catch (error) {
    console.error("window control failed", error);
  }
}

async function startWindowDrag() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch (error) {
    console.error("window drag failed", error);
  }
}

function App() {
  const [page, setPage] = useState<"home" | "watch" | "collection">("home");
  const [searchOpen, setSearchOpen] = useState(false);
  const [collectionTab, setCollectionTab] = useState<"downloads">("downloads");
  const [selectedEpisode, setSelectedEpisode] = useState<number | null>(1);
  const [selectedSeason, setSelectedSeason] = useState<number | null>(1);
  const [initialResumeSeconds, setInitialResumeSeconds] = useState(0);
  const [resumeReady, setResumeReady] = useState(false);
  const [query, setQuery] = useState("Attack on Titan");
  const [submittedQuery, setSubmittedQuery] = useState("Attack on Titan");
  const [downloadMessage, setDownloadMessage] = useState<string | null>(null);
  const [downloadQueue, setDownloadQueue] = useState<AnimeDownloadRecord[]>([]);
  const {
    latestItems,
    loading: catalogLoading,
    error: catalogError,
    search,
    refreshLatest,
  } = useAnimeCatalog();

  const detailsRequest = useMemo(
    () => ({
      tmdbId: 94605,
      anilistId: null,
      malId: null,
      title: submittedQuery,
      overview: null,
      posterUrl: null,
      backdropUrl: null,
      genres: ["Animation"],
      releaseYear: null,
      status: null,
      hasAnimationGenre: true,
      originalLanguage: "ja",
      originCountries: ["JP"],
    }),
    [submittedQuery],
  );

  const { details, loading: detailsLoading, error: detailsError } = useAnimeDetails(detailsRequest);

  const identityKey = useMemo(() => {
    if (!details) return null;
    return buildIdentityKey(details.identity);
  }, [details]);

  useEffect(() => {
    if (page !== "watch" || !details || !identityKey) return;
    let cancelled = false;
    setResumeReady(false);
    animeGetResumeProgress({
      identity: details.identity,
      seasonNumber: null,
      episodeNumber: null,
    })
      .then((resume) => {
        if (cancelled) return;
        const targetEpisode = resume?.episodeNumber ?? 1;
        const targetSeason = resume?.seasonNumber ?? 1;
        setSelectedSeason(targetSeason);
        setSelectedEpisode(targetEpisode);
        setInitialResumeSeconds(resume?.progressSeconds ?? 0);
        setResumeReady(true);
      })
      .catch(() => {
        if (!cancelled) {
          setSelectedSeason(1);
          setSelectedEpisode(1);
          setInitialResumeSeconds(0);
          setResumeReady(true);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [page, identityKey, details]);

  const episodesRequest = useMemo(() => {
    if (!details) return null;
    const preferredSeasonNumber = parseSeasonHint(details.identity.canonicalTitle);
    return {
      identity: details.identity,
      isMovie: false,
      tmdbEpisodes: [],
      anilistEpisodeCount: details.totalEpisodes ?? 12,
      releasedEpisodeCount: details.releasedEpisodeCount,
      preferredSeasonNumber,
    };
  }, [details]);

  const { episodes, loading: episodesLoading, error: episodesError } = useAnimeEpisodes(episodesRequest);

  const availableSeasons = useMemo(() => {
    const unique = Array.from(new Set(episodes.map((episode) => episode.seasonNumber))).sort((a, b) => a - b);
    return unique;
  }, [episodes]);

  const visibleEpisodes = useMemo(() => {
    if (selectedSeason == null) return episodes;
    return episodes.filter((episode) => episode.seasonNumber === selectedSeason);
  }, [episodes, selectedSeason]);

  useEffect(() => {
    if (availableSeasons.length === 0) return;
    if (selectedSeason != null && availableSeasons.includes(selectedSeason)) return;

    const hinted = parseSeasonHint(details?.identity.canonicalTitle);
    const nextSeason = hinted && availableSeasons.includes(hinted) ? hinted : availableSeasons[0];
    setSelectedSeason(nextSeason);

    const firstEpisode = episodes.find((episode) => episode.seasonNumber === nextSeason);
    if (firstEpisode) {
      setSelectedEpisode(firstEpisode.canonicalEpisodeNumber);
    }
  }, [availableSeasons, selectedSeason, episodes, details]);

  const playbackRequest = useMemo(() => {
    if (!details || !resumeReady) return null;
    return {
      animeId: details.identity,
      translationMode: "sub" as const,
      movie: false,
      seasonNumber: selectedSeason,
      episodeNumber: selectedEpisode,
      resumeSeconds: initialResumeSeconds,
    };
  }, [details, selectedEpisode, selectedSeason, resumeReady, initialResumeSeconds]);

  const {
    translationMode,
    setTranslationMode,
    source,
    sources,
    selectedSourceId,
    setSelectedSourceId,
    error: playbackError,
    loading,
  } =
    useAnimePlaybackSession(playbackRequest);
  const playbackLabel = sources.find((item) => item.id === selectedSourceId)?.label ?? "Streaming";

  const handleAddToDownloads = async () => {
    if (!details || !source) return;

    const currentSessionRequest = {
      animeId: details.identity,
      translationMode,
      movie: false,
      seasonNumber: selectedSeason,
      episodeNumber: selectedEpisode,
      resumeSeconds: initialResumeSeconds,
    };

    try {
      const response = await animePrepareDownload({
        request: currentSessionRequest,
        source,
      });
      const downloadId = crypto.randomUUID();
      await animeDownloadsEnqueue({ downloadId, payload: response.payload });
      await refreshDownloads();
      setDownloadMessage(`Queued: ${response.payload.fileName}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to queue download";
      setDownloadMessage(message);
    }
  };

  const handleRetryDownload = async (id: string) => {
    const target = downloadQueue.find((item) => item.id === id);
    if (!target) return;
    await animeDownloadsEnqueue({ downloadId: id, payload: target.payload });
    void refreshDownloads();
  };

  const handleCancelDownload = async (id: string) => {
    await animeDownloadsCancel({ downloadId: id });
    void refreshDownloads();
  };

  const handleRemoveDownload = async (id: string) => {
    await animeDownloadsRemove({ downloadId: id });
    void refreshDownloads();
  };

  const handleOpenDownloadFolder = async (outputPath: string | null) => {
    if (!outputPath) return;
    const slashIndex = Math.max(outputPath.lastIndexOf("/"), outputPath.lastIndexOf("\\"));
    const folderPath = slashIndex > 0 ? outputPath.slice(0, slashIndex) : outputPath;
    try {
      await openPath(folderPath);
    } catch {
      setDownloadMessage("Could not open download folder");
    }
  };

  const refreshDownloads = useCallback(async () => {
    try {
      const result = await animeDownloadsList();
      setDownloadQueue(result.downloads);
    } catch {
      // ignore
    }
  }, []);

  const activeDownloadCount = useMemo(
    () => downloadQueue.filter((item) => item.status === "downloading" || item.status === "queued").length,
    [downloadQueue],
  );

  useEffect(() => {
    const migrateLegacyQueue = async () => {
      const MIGRATION_FLAG = "ether.downloadQueue.migrated";
      if (localStorage.getItem(MIGRATION_FLAG)) return;

      const raw = localStorage.getItem("ether.downloadQueue");
      if (raw) {
        try {
          const legacy = JSON.parse(raw) as Array<{ id?: string; payload?: unknown }>;
          for (const entry of legacy) {
            if (!entry?.payload) continue;
            const downloadId = entry.id ?? crypto.randomUUID();
            await animeDownloadsEnqueue({
              downloadId,
              payload: entry.payload as never,
            });
          }
        } catch {
          // ignore malformed legacy data
        }
      }

      localStorage.removeItem("ether.downloadQueue");
      localStorage.setItem(MIGRATION_FLAG, "1");
    };

    void migrateLegacyQueue().finally(() => {
      void refreshDownloads();
    });

    const timer = window.setInterval(() => {
      void refreshDownloads();
    }, 2000);
    return () => window.clearInterval(timer);
  }, [refreshDownloads]);

  const { timings } = useAnimeSkipState(
    details?.identity.malId
      ? {
          malId: details.identity.malId,
          episodeNumber: selectedEpisode ?? 1,
        }
      : null,
  );

  return (
    <main className="anime-shell">
      <div className="titlebar">
        <div
          className="titlebar-drag"
          onMouseDown={(event) => {
            if (event.button !== 0) return;
            void startWindowDrag();
          }}
        >
        </div>
        <div className="titlebar-actions">
          <button
            type="button"
            className="danger"
            aria-label="Close window"
            onClick={() => void windowControl("close")}
          >
            <svg viewBox="0 0 12 12" aria-hidden="true" focusable="false">
              <path d="M2 2L10 10M10 2L2 10" />
            </svg>
          </button>
        </div>
      </div>

      <header className="top-grid">
        <button
          type="button"
          className="top-cell brand brand-button"
          onClick={() => {
            setPage("home");
            void refreshLatest();
          }}
        >
          ETHER
        </button>
        <button type="button" className="top-cell nav-button" onClick={() => setSearchOpen((value) => !value)}>
          <span className="nav-inline-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" focusable="false">
              <circle cx="11" cy="11" r="6.5" />
              <path d="M16 16L21 21" />
            </svg>
          </span>
          <span className="nav-inline-text">SEARCH</span>
        </button>
        <button
          type="button"
          className="top-cell nav-button"
          onClick={() => {
            setPage("collection");
            setSearchOpen(false);
          }}
        >
          <span className="nav-inline-text">COLLECTION</span>
          {activeDownloadCount > 0 ? (
            <span className="nav-badge" aria-label={`${activeDownloadCount} active downloads`}>
              {activeDownloadCount}
            </span>
          ) : null}
        </button>
        <div className="top-cell">SESSION</div>
        <div className="top-cell">SETTINGS</div>
      </header>

      {searchOpen ? (
        <section className="search-panel">
          <form
            className="search-row"
            onSubmit={(event) => {
              event.preventDefault();
              const q = query.trim() || "Attack on Titan";
              setSubmittedQuery(q);
              void search(q);
              setPage("watch");
              setSearchOpen(false);
            }}
          >
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search anime title"
            />
            <button type="submit">Find Anime</button>
          </form>
        </section>
      ) : null}

      <section className="anime-hero">
        <div className="anime-hero-inner">
          <div className="anime-kicker">Calm • Anime Session</div>
          <h1>{page === "watch" ? (details?.identity.canonicalTitle ?? "Loading title...") : "Find and Watch Anime"}</h1>
          <p>
            {page === "watch"
              ? (details?.overview ?? "Pick an episode and start watching.")
              : "Browse latest anime releases or search by title."}
          </p>
          {page === "watch" ? (
          <div className="anime-meta-row">
            <AnimeBadge label="Anime" />
            <AnimeBadge label={details?.status ?? "Unknown status"} />
            <AnimeBadge label={`${details?.releaseYear ?? "-"}`} />
            <AnimeBadge label={translationMode.toUpperCase()} />
          </div>
          ) : null}
        </div>
      </section>

      {page === "home" ? (
        <section className="anime-grid">
          <article className="anime-card anime-card-wide">
            <h2>Latest Releases</h2>
            {catalogLoading ? <p className="muted">Loading latest anime...</p> : null}
            {catalogError ? <PlaybackErrorState message={catalogError} /> : null}
            <div className="catalog-grid">
              {latestItems.map((item) => (
                <button
                  key={item.anilistId}
                  className="catalog-card"
                  type="button"
                  onClick={() => {
                    setQuery(item.title);
                    setSubmittedQuery(item.title);
                    setPage("watch");
                  }}
                >
                  <div className="catalog-title">{item.title}</div>
                  <div className="catalog-meta">{item.year ?? "-"} • {item.episodes ?? "?"} eps</div>
                </button>
              ))}
            </div>
          </article>
        </section>
      ) : null}

      {page === "watch" ? (
      <section className="anime-grid">
        <article className="anime-card anime-card-wide watch-layout">
          <div className="watch-main">
            {source?.url && details ? (
                <PlaybackSurface
                  source={source}
                  identity={details.identity}
                  seasonNumber={selectedSeason}
                  episodeNumber={selectedEpisode}
                  resumeSeconds={initialResumeSeconds}
                />
              ) : (
              <div className="player-placeholder">Resolve source to start playback</div>
            )}
          </div>

          <aside className="watch-sidebar">
            <h2>{details?.identity.canonicalTitle ?? "Anime"}</h2>
            <p className="muted">{details?.overview ?? "No overview available."}</p>
            <div className="session-panel">
              <h3>Session</h3>
              <TranslationToggle value={translationMode} onChange={setTranslationMode} />
              <label className="source-picker">
                <span className="muted">Source</span>
                <select
                  value={selectedSourceId ?? ""}
                  onChange={(event) => setSelectedSourceId(event.target.value)}
                  disabled={sources.length === 0}
                >
                  {sources.map((option) => (
                    <option key={option.id} value={option.id}>{option.label}</option>
                  ))}
                </select>
              </label>
              <div className="session-meta-grid">
                <p className="muted"><strong>Season:</strong> {selectedSeason ?? 1}</p>
                <p className="muted"><strong>Episode:</strong> {selectedEpisode ?? 1}</p>
                <p className="muted"><strong>Mode:</strong> {translationMode.toUpperCase()}</p>
                <p className="muted"><strong>Source:</strong> {playbackLabel}</p>
              </div>
              <label className="source-picker">
                <span className="muted">Season Selector</span>
                <select
                  value={selectedSeason ?? 1}
                  onChange={(event) => {
                    const nextSeason = Number(event.target.value);
                    setSelectedSeason(nextSeason);
                    const firstEpisode = episodes.find((episode) => episode.seasonNumber === nextSeason);
                    setSelectedEpisode(firstEpisode?.canonicalEpisodeNumber ?? 1);
                    setInitialResumeSeconds(0);
                  }}
                  disabled={availableSeasons.length === 0}
                >
                  {availableSeasons.map((seasonNumber) => (
                    <option key={seasonNumber} value={seasonNumber}>
                      Season {seasonNumber}
                    </option>
                  ))}
                </select>
              </label>
              <button
                type="button"
                className="download-action"
                onClick={() => void handleAddToDownloads()}
                disabled={!source}
              >
                Add To Downloads
              </button>
              {downloadMessage ? <p className="muted download-message">{downloadMessage}</p> : null}
              {loading ? <div className="loading-spinner" aria-label="Loading" /> : null}
              <PlaybackErrorState message={playbackError ?? detailsError} />
            </div>

          </aside>
        </article>

        <article className="anime-card anime-card-wide">
          <h2>Episodes</h2>
          {detailsLoading || episodesLoading ? <p className="muted">Loading episode map...</p> : null}
          {episodesError ? <PlaybackErrorState message={episodesError} /> : null}
          <EpisodeSelector
            episodes={visibleEpisodes}
            selectedEpisodeNumber={selectedEpisode}
            onSelect={(episode) => {
              setSelectedEpisode(episode.canonicalEpisodeNumber);
              setSelectedSeason(episode.seasonNumber);
              setInitialResumeSeconds(0);
              if (details) {
                void animeSetLastEpisode({
                  identity: details.identity,
                  seasonNumber: episode.seasonNumber,
                  episodeNumber: episode.canonicalEpisodeNumber,
                });
              }
            }}
          />
        </article>
      </section>
      ) : null}

      {page === "collection" ? (
        <section className="anime-grid">
          <article className="anime-card anime-card-wide">
            <div className="collection-tabs">
              <button
                type="button"
                className={`queue-action ${collectionTab === "downloads" ? "is-active" : ""}`}
                onClick={() => setCollectionTab("downloads")}
              >
                Downloads
              </button>
            </div>

            {collectionTab === "downloads" ? (
              <div className="download-queue-list">
                {downloadQueue.map((item) => {
                  const percent =
                    item.totalBytes > 0
                      ? Math.min(100, Math.round((item.bytesDownloaded / item.totalBytes) * 100))
                      : null;
                  const downloadedMb = item.bytesDownloaded / (1024 * 1024);
                  const totalMb = item.totalBytes / (1024 * 1024);
                  return (
                    <div key={item.id} className="download-row">
                      <p className="download-row-title" title={item.payload.fileName}>{item.payload.fileName}</p>
                      <p className="muted download-row-meta">
                        {item.status}
                        {item.status === "downloading" && percent != null ? ` • ${percent}%` : ""}
                        {item.bytesDownloaded > 0
                          ? ` • ${downloadedMb.toFixed(1)}${item.totalBytes > 0 ? `/${totalMb.toFixed(1)}` : ""} MB`
                          : ""}
                      </p>
                      {item.status === "downloading" ? (
                        <div className="download-progress">
                          <div
                            className={`download-progress-bar${percent == null ? " indeterminate" : ""}`}
                            style={percent != null ? { width: `${percent}%` } : undefined}
                          />
                        </div>
                      ) : null}
                      <p className="muted download-row-meta">{item.payload.mediaName}</p>
                      <div className="download-row-actions">
                        {item.status === "failed" || item.status === "cancelled" ? (
                          <button type="button" className="queue-action" onClick={() => void handleRetryDownload(item.id)}>
                            Retry
                          </button>
                        ) : null}
                        {item.status === "queued" || item.status === "downloading" ? (
                          <button type="button" className="queue-action" onClick={() => void handleCancelDownload(item.id)}>
                            Cancel
                          </button>
                        ) : null}
                        {item.status === "completed" ? (
                          <button type="button" className="queue-action" onClick={() => void handleOpenDownloadFolder(item.outputPath)}>
                            Open Folder
                          </button>
                        ) : null}
                        <button type="button" className="queue-action" onClick={() => void handleRemoveDownload(item.id)}>
                          Remove
                        </button>
                      </div>
                      {item.error ? <p className="muted download-message">{item.error}</p> : null}
                    </div>
                  );
                })}
                {downloadQueue.length === 0 ? <p className="muted">No downloaded anime yet.</p> : null}
              </div>
            ) : null}
          </article>
        </section>
      ) : null}

      <SkipPrompt segment={timings?.segments?.[0] ?? null} onSkip={() => {}} />
    </main>
  );
}

export default App;
