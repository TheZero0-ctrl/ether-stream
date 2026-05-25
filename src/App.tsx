import { useEffect, useMemo, useRef, useState } from "react";
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
import { animeGetResumeProgress, animeSetLastEpisode, animeUpdateProgress } from "./features/anime/api";
import type { AnimeIdentity } from "./features/anime/types";
import "./App.css";

function buildIdentityKey(identity: { tmdbId: number | null; anilistId: number | null; malId: number | null }) {
  return `tmdb:${identity.tmdbId ?? "none"}|anilist:${identity.anilistId ?? "none"}|mal:${identity.malId ?? "none"}`;
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
  const isHls = source.playbackKind === "hls" || source.url.includes(".m3u8");
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
          video.src = source.url;
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
          hls.loadSource(source.url);
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
  }, [isHls, source.url, hlsFailed]);

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
  }, [source.url, resumeSeconds]);

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
  }, [identity, seasonNumber, episodeNumber, source.url]);

  if (source.playbackKind === "webviewRemote" && !isHls) {
    return (
      <iframe
        className="player-frame"
        src={source.url}
        title="Anime playback"
        allow="autoplay; fullscreen; picture-in-picture"
      />
    );
  }

  if (hlsFailed) {
    return (
      <iframe
        className="player-frame"
        src={source.url}
        title="Anime playback fallback"
        allow="autoplay; fullscreen; picture-in-picture"
      />
    );
  }

  return (
    <video ref={videoRef} className="player-video" controls autoPlay playsInline src={isHls ? undefined : source.url}>
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
  const [page, setPage] = useState<"home" | "watch">("home");
  const [searchOpen, setSearchOpen] = useState(false);
  const [selectedEpisode, setSelectedEpisode] = useState<number | null>(1);
  const [selectedSeason, setSelectedSeason] = useState<number | null>(1);
  const [initialResumeSeconds, setInitialResumeSeconds] = useState(0);
  const [resumeReady, setResumeReady] = useState(false);
  const [query, setQuery] = useState("Attack on Titan");
  const [submittedQuery, setSubmittedQuery] = useState("Attack on Titan");
  const { items, loading: catalogLoading, error: catalogError, search } = useAnimeCatalog();

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
    return {
      identity: details.identity,
      isMovie: false,
      tmdbEpisodes: [],
      anilistEpisodeCount: details.totalEpisodes ?? 12,
    };
  }, [details]);

  const { episodes, loading: episodesLoading, error: episodesError } = useAnimeEpisodes(episodesRequest);

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

  const { translationMode, setTranslationMode, source, error: playbackError, loading } =
    useAnimePlaybackSession(playbackRequest);

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
          <div className="titlebar-label">Ether</div>
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
        <button type="button" className="top-cell brand brand-button" onClick={() => setPage("home")}>
          ETHER<br />ANIME
        </button>
        <button type="button" className="top-cell nav-button" onClick={() => setSearchOpen((value) => !value)}>
          <span className="nav-inline-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" focusable="false">
              <circle cx="11" cy="11" r="6.5" />
              <path d="M16 16L21 21" />
            </svg>
          </span>
          <span>SEARCH</span>
        </button>
        <div className="top-cell">COLLECTION</div>
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
          <div className="anime-kicker">Calm Mode • Anime Session</div>
          <h1>{page === "watch" ? (details?.identity.canonicalTitle ?? "Loading title...") : "Find and Watch Anime"}</h1>
          <p>
            {page === "watch"
              ? (details?.overview ?? "Pick an episode and start watching.")
              : "Browse latest anime releases or search by title, then open episode view and continue where you left off."}
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
              {items.map((item) => (
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
              <p className="muted">Episode {selectedEpisode ?? 1}</p>
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
            episodes={episodes}
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

      <SkipPrompt segment={timings?.segments?.[0] ?? null} onSkip={() => {}} />
    </main>
  );
}

export default App;
