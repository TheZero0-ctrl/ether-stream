# Anime Data Model

## Purpose

Streambert currently mixes:

- TMDB item shapes
- AniList response fragments
- page-specific derived state
- provider-specific source data

Ether should define canonical anime models in Rust and expose stable frontend types derived from them.

## Core Identity Model

### `AnimeIdentity`

```text
AnimeIdentity
  media_kind: AnimeSeries | AnimeMovie
  tmdb_id: Option<i64>
  anilist_id: Option<i64>
  mal_id: Option<i64>
  canonical_title: String
  romaji_title: Option<String>
  english_title: Option<String>
  native_title: Option<String>
  title_aliases: Vec<String>
```

Notes:

- `tmdb_id` is important because discovery and most entry flows are TMDB-first.
- `anilist_id` and `mal_id` are needed for anime enrichment and AniSkip.
- `title_aliases` should power provider search candidate generation in the backend.

## Presentation Model

### `AnimeDetails`

```text
AnimeDetails
  identity: AnimeIdentity
  overview: Option<String>
  poster_url: Option<String>
  backdrop_url: Option<String>
  genres: Vec<String>
  score: Option<f32>
  release_year: Option<i32>
  status: Option<String>
  source_confidence: AnimeSourceConfidence
```

`source_confidence` is optional but useful for debugging or future diagnostics.

## Season and Episode Model

### `AnimeSeason`

```text
AnimeSeason
  season_number: i32
  title: String
  episode_count: Option<i32>
  source_kind: Tmdb | Anilist | Hybrid | ProviderDerived
  episodes: Vec<AnimeEpisode>
```

### `AnimeEpisode`

```text
AnimeEpisode
  display_episode_number: i32
  canonical_episode_number: i32
  season_number: i32
  title: Option<String>
  overview: Option<String>
  runtime_minutes: Option<i32>
  air_date: Option<String>
  tmdb_reference: Option<TmdbEpisodeRef>
  provider_reference: Option<ProviderEpisodeRef>
```

### `TmdbEpisodeRef`

```text
TmdbEpisodeRef
  season_number: i32
  episode_number: i32
```

### `ProviderEpisodeRef`

```text
ProviderEpisodeRef
  provider: AnimeProvider
  provider_show_id: Option<String>
  provider_episode_token: Option<String>
  provider_season_number: Option<i32>
  provider_episode_number: Option<String>
```

## Translation Mode

### `AnimeTranslationMode`

```text
AnimeTranslationMode
  Sub
  Dub
```

This should be persisted as user preference, but availability should be resolved per title or per episode.

## Playback Source Model

### `AnimePlaybackRequest`

```text
AnimePlaybackRequest
  anime_id: AnimeIdentity
  translation_mode: AnimeTranslationMode
  movie: bool
  season_number: Option<i32>
  episode_number: Option<i32>
  resume_seconds: Option<u64>
```

### `AnimePlaybackSource`

```text
AnimePlaybackSource
  provider: AnimeProvider
  playback_kind: WebviewRemote | LocalProxy | DirectVideo | Hls
  url: String
  referer: Option<String>
  subtitle_candidates: Vec<DiscoveredSubtitle>
  is_downloadable: bool
```

## Skip Model

### `SkipSegment`

```text
SkipSegment
  kind: Intro | Outro
  start_seconds: f64
  end_seconds: f64
```

### `AnimeSkipTimings`

```text
AnimeSkipTimings
  mal_id: i64
  episode_number: i32
  segments: Vec<SkipSegment>
```

## Download Preparation Model

### `AnimeDownloadPayload`

```text
AnimeDownloadPayload
  media_name: String
  tmdb_id: Option<i64>
  season_number: Option<i32>
  episode_number: Option<i32>
  playback_url: String
  referer: Option<String>
  subtitle_candidates: Vec<ResolvedSubtitle>
```

The key point is that downloads should consume a normalized payload prepared by Rust, not raw page state.

## Settings Model

Anime-related settings should be explicit:

```text
AnimeSettings
  default_translation_mode: AnimeTranslationMode
  intro_skip_mode: Off | Auto | Manual
  preferred_subtitle_language: String
```

## Design Rule

There should be one canonical anime episode model in Ether.

Do not keep separate hidden concepts like:

- display episode number
- TMDB episode number
- provider episode number

without a shared backend model that explains how they relate.
