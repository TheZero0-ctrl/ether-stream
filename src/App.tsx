import { useMemo, useState } from "react";
import {
  AnimeBadge,
  EpisodeSelector,
  PlaybackErrorState,
  SkipPrompt,
  TranslationToggle,
} from "./features/anime/components";
import {
  useAnimeDetails,
  useAnimeEpisodes,
  useAnimePlaybackSession,
  useAnimeSkipState,
} from "./features/anime/hooks";
import "./App.css";

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
  const [selectedEpisode, setSelectedEpisode] = useState<number | null>(1);

  const detailsRequest = useMemo(
    () => ({
      tmdbId: 94605,
      anilistId: 16498,
      malId: 16498,
      title: "Attack on Titan",
      overview: "Humanity survives behind walls while Titans roam beyond.",
      posterUrl: null,
      backdropUrl: null,
      genres: ["Animation", "Action", "Drama"],
      releaseYear: 2013,
      status: "Finished",
      hasAnimationGenre: true,
      originalLanguage: "ja",
      originCountries: ["JP"],
    }),
    [],
  );

  const { details, loading: detailsLoading, error: detailsError } = useAnimeDetails(detailsRequest);

  const episodesRequest = useMemo(() => {
    if (!details) return null;
    return {
      identity: details.identity,
      isMovie: false,
      tmdbEpisodes: [
        {
          tmdbSeasonNumber: 1,
          tmdbEpisodeNumber: 1,
          anilistEpisodeNumber: 1,
          title: "To You, in 2000 Years",
          runtimeMinutes: 24,
        },
        {
          tmdbSeasonNumber: 1,
          tmdbEpisodeNumber: 2,
          anilistEpisodeNumber: 2,
          title: "That Day",
          runtimeMinutes: 24,
        },
      ],
      anilistEpisodeCount: 2,
    };
  }, [details]);

  const { episodes, loading: episodesLoading, error: episodesError } = useAnimeEpisodes(episodesRequest);

  const playbackRequest = useMemo(() => {
    if (!details) return null;
    return {
      animeId: details.identity,
      translationMode: "sub" as const,
      movie: false,
      seasonNumber: 1,
      episodeNumber: selectedEpisode,
      resumeSeconds: 0,
    };
  }, [details, selectedEpisode]);

  const { translationMode, setTranslationMode, source, error: playbackError, resolve, loading } =
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
        <div className="top-cell brand">ETHER<br />ANIME</div>
        <div className="top-cell">DISCOVER</div>
        <div className="top-cell">COLLECTION</div>
        <div className="top-cell">SESSION</div>
        <div className="top-cell">SETTINGS</div>
      </header>

      <section className="anime-hero">
        <div className="anime-hero-inner">
          <div className="anime-kicker">Calm Mode • Anime Session</div>
          <h1>{details?.identity.canonicalTitle ?? "Loading title..."}</h1>
          <p>{details?.overview ?? "Gathering canonical metadata from Rust services."}</p>
          <div className="anime-meta-row">
            <AnimeBadge label="Anime" />
            <AnimeBadge label={details?.status ?? "Unknown status"} />
            <AnimeBadge label={`${details?.releaseYear ?? "-"}`} />
            <AnimeBadge label={translationMode.toUpperCase()} />
          </div>
        </div>
      </section>

      <section className="anime-grid">
        <article className="anime-card">
          <h2>Playback</h2>
          <TranslationToggle value={translationMode} onChange={setTranslationMode} />
          <button className="primary-action" type="button" onClick={resolve} disabled={loading}>
            {loading ? "Resolving..." : "Play / Resume"}
          </button>
          {source?.url ? <p className="muted">Source ready.</p> : <p className="muted">No source resolved yet.</p>}
          <PlaybackErrorState message={playbackError ?? detailsError} />
        </article>

        <article className="anime-card">
          <h2>Episodes</h2>
          {detailsLoading || episodesLoading ? <p className="muted">Loading episode map...</p> : null}
          {episodesError ? <PlaybackErrorState message={episodesError} /> : null}
          <EpisodeSelector
            episodes={episodes}
            selectedEpisodeNumber={selectedEpisode}
            onSelect={(episode) => setSelectedEpisode(episode.canonicalEpisodeNumber)}
          />
        </article>

        <article className="anime-card anime-card-wide">
          <h2>See It In Action</h2>
          <p className="muted">
            Canonical anime identity is backed by Rust services, with episode mapping and translation
            mode controls surfaced through thin frontend contracts.
          </p>
        </article>
      </section>

      <SkipPrompt segment={timings?.segments?.[0] ?? null} onSkip={() => {}} />
    </main>
  );
}

export default App;
