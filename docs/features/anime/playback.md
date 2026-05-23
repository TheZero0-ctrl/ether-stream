# Anime Playback

## Goal

Anime playback in Ether should support:

- episode and movie playback
- sub/dub switching
- progress tracking
- intro/outro skipping where supported
- subtitle attachment and discovery
- download preparation

## Current Streambert Behavior

Streambert uses a webview-based playback path and relies on main-process interception and provider resolution.

Important behavior today:

- anime source defaults to `allmanga`
- some resolved sources return direct media URLs
- a local proxy player is used for direct video or HLS when needed
- `.m3u8` and `.vtt` URLs are intercepted during playback
- AniSkip only works for anime when using the AllManga source

## Ether Playback Model

Anime playback should sit on top of a shared playback subsystem, but with anime-specific inputs.

## Playback Modes

### 1. Remote webview-backed playback

Use when the resolved provider requires its own page or source environment.

### 2. Local proxy playback

Use when the backend resolves a direct media URL but browser playback needs referer/header control or HLS adaptation.

### 3. Local/offline playback

Use downloaded files with shared desktop playback behavior.

## Playback Workflow

1. frontend requests anime playback
2. backend resolves canonical episode/movie source
3. backend returns playback payload
4. frontend mounts shared player view with minimal source-specific knowledge
5. backend and player exchange progress and state events

## Sub/Dub Behavior

### Requirements

- user can switch translation mode from the player UI
- preference can be remembered globally
- availability is resolved per show or episode
- failed dub resolution should fall back cleanly to sub or show an explicit error

### Design Rule

Sub/dub is part of anime playback state, not just a button that mutates page-local state.

## Progress Tracking

Progress should be shared with the global library/progress subsystem.

Anime-specific requirements:

- persist progress per canonical episode key
- persist resume seconds for local or remote playback
- support watched thresholds for auto-marking watched

The frontend should not be the source of truth for progress persistence.

## AniSkip Integration

### Support Rules

AniSkip should only activate when:

- anime identity is valid
- `mal_id` is known
- playback source supports time-based seeking
- user setting is `auto` or `manual`

### Modes

- `off`
  no skip handling
- `auto`
  backend/player seeks automatically when current time enters intro/outro segment
- `manual`
  frontend shows skip affordance, backend provides active segment state

### Recommended split

- Rust resolves and caches skip timings
- shared playback state tracks current playback time
- UI only renders skip prompt and forwards user actions

## Subtitle Handling

Anime playback may surface subtitle information from:

- provider playback discovery
- external subtitle providers
- downloaded subtitle files

Backend should normalize subtitle candidates into a shared subtitle model.

## Download Preparation

Playback should expose a stable `prepare for download` path.

Instead of handing raw page state directly into a download modal, backend should provide:

- final media URL
- required referer/header state
- media identity
- subtitle candidates
- filename metadata

## Event Model

Suggested runtime events:

- `anime-playback-ready`
- `anime-playback-failed`
- `anime-progress-updated`
- `anime-skip-timings-loaded`
- `anime-skip-segment-active`
- `anime-subtitle-candidates-updated`

## Important Implementation Notes

- Keep provider-specific behavior out of React components.
- Keep skip logic reusable and source-aware.
- Do not tie anime playback architecture to one provider forever.
- Avoid singleton/global playback resolution state where possible.
