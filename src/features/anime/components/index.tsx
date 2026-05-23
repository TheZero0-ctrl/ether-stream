import type { AnimeEpisode, SkipSegment } from "../types";

type BadgeProps = { label: string };

export function AnimeBadge({ label }: BadgeProps) {
  return <span className="anime-badge">{label}</span>;
}

type EpisodeSelectorProps = {
  episodes: AnimeEpisode[];
  selectedEpisodeNumber: number | null;
  onSelect: (episode: AnimeEpisode) => void;
};

export function EpisodeSelector({ episodes, selectedEpisodeNumber, onSelect }: EpisodeSelectorProps) {
  return (
    <div className="episode-selector">
      {episodes.map((episode) => {
        const selected = selectedEpisodeNumber === episode.canonicalEpisodeNumber;
        return (
          <button
            type="button"
            key={`${episode.seasonNumber}-${episode.canonicalEpisodeNumber}`}
            className={`episode-chip${selected ? " is-active" : ""}`}
            onClick={() => onSelect(episode)}
          >
            <span className="episode-chip-number">E{episode.displayEpisodeNumber}</span>
            <span className="episode-chip-title">{episode.title ?? "Untitled"}</span>
          </button>
        );
      })}
    </div>
  );
}

type TranslationToggleProps = {
  value: "sub" | "dub";
  onChange: (value: "sub" | "dub") => void;
};

export function TranslationToggle({ value, onChange }: TranslationToggleProps) {
  return (
    <div className="translation-toggle" role="group" aria-label="Translation mode">
      <button
        type="button"
        className={value === "sub" ? "is-active" : ""}
        onClick={() => onChange("sub")}
      >
        Sub
      </button>
      <button
        type="button"
        className={value === "dub" ? "is-active" : ""}
        onClick={() => onChange("dub")}
      >
        Dub
      </button>
    </div>
  );
}

type SkipPromptProps = {
  segment: SkipSegment | null;
  onSkip: () => void;
};

export function SkipPrompt({ segment, onSkip }: SkipPromptProps) {
  if (!segment) return null;

  return (
    <aside className="skip-prompt">
      <div>
        Skip {segment.kind} ({Math.round(segment.startSeconds)}s - {Math.round(segment.endSeconds)}s)
      </div>
      <button type="button" onClick={onSkip}>
        Skip now
      </button>
    </aside>
  );
}

type PlaybackErrorStateProps = { message: string | null };

export function PlaybackErrorState({ message }: PlaybackErrorStateProps) {
  if (!message) return null;
  return <div className="playback-error">{message}</div>;
}
