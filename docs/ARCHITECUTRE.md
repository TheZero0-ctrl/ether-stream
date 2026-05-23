# Ether Architecture

## Purpose

Ether is a desktop-first rewrite intended to recreate and improve the application currently implemented in `/home/ankit/projects/opensource/streambert`.

The goal is not a line-by-line port. The goal is to preserve the strong product capabilities from Streambert while improving:

- architecture boundaries
- native integration quality
- long-term maintainability
- typed contracts between UI and backend
- performance and memory usage

## Product Direction

Ether is being built as:

- `Tauri` desktop shell
- `Rust` native/core runtime
- `React + TypeScript` frontend

This means:

- Rust owns native capabilities and business logic
- React owns interface rendering and user interaction
- Tauri commands and events are the contract boundary between them

## High-Level Runtime Layers

### 1. Frontend Layer

Current location:

- `src/`

Responsibilities:

- render pages and components
- manage view state
- call typed Tauri commands
- subscribe to backend events
- coordinate navigation and user workflows

The frontend should stay thin. It should not contain the real app engine.

### 2. Application Core Layer

Current location:

- `src-tauri/src/`

Responsibilities:

- persistence
- secrets management
- media metadata services
- subtitle services
- downloads orchestration
- playback/session orchestration
- update and backup logic

This layer replaces the Electron main-process service sprawl from Streambert with a more explicit Rust service model.

### 3. Platform Layer

Responsibilities:

- OS keychain and secure storage
- filesystem access
- native window control
- updater integration
- process spawning
- background task communication

This stays behind the application core and should not leak raw platform details into the React UI.

## Target Architecture Principles

### Backend-First Logic

In Streambert, a lot of important logic lives in the renderer and is coordinated through Electron IPC. In Ether, the important behavior should move into Rust early.

Examples:

- do not keep download orchestration in the UI
- do not keep subtitle provider logic in the UI
- do not keep storage rules in the UI
- do not keep source resolution in the UI

The UI should ask for state and issue commands. Rust should perform the work.

### Typed Boundaries

Every command/event contract between React and Rust should have a typed frontend model and a matching Rust payload shape.

This reduces drift and helps keep the app maintainable as features grow.

### Feature-Oriented Modules

Ether should be split by feature/service area rather than by large generic utility files.

Recommended backend areas:

- `app_state`
- `settings`
- `secrets`
- `metadata`
- `search`
- `library`
- `downloads`
- `subtitles`
- `playback`
- `backups`
- `updates`

Recommended frontend areas:

- `features/library`
- `features/search`
- `features/player`
- `features/downloads`
- `features/settings`
- `features/setup`
- `components/ui`
- `lib/tauri`
- `types/`

## Major Functional Areas

### Metadata and Discovery

Ether will still need:

- TMDB for search, trending, movie/TV metadata, images
- AniList for anime metadata and season modeling

Target design:

- provider clients implemented in Rust
- normalized domain models returned to the frontend
- explicit caching layer in Rust or SQLite-backed persistence

The frontend should not know provider-specific response structures more than necessary.

### Playback

Playback is one of the most important architectural decisions.

Likely target approach:

1. remote streaming playback still uses a webview-backed approach when required
2. local/offline playback can become more native over time
3. playback/session state should be coordinated by Rust rather than scattered across frontend components

Streambert depends heavily on webview session interception. Ether should isolate this into a dedicated playback service instead of letting playback details leak into page components.

### Downloads

Downloads should be treated as a real backend subsystem.

Target responsibilities:

- queue management
- download process spawning or native implementation
- progress tracking
- persistence
- completion state transitions
- file validation
- subtitle attachment

The frontend should consume a typed download state model and listen to progress events.

### Subtitles

Subtitle work should stay in Rust.

Target responsibilities:

- provider queries
- ZIP extraction
- language normalization
- subtitle file download and storage
- attachment to local media/download entries

### Persistence

Streambert currently spreads state across localStorage, secure files, and JSON registries. Ether should simplify this.

Recommended direction:

- `SQLite` for structured state
- config files for a small set of app-level settings if needed
- OS keychain for secrets
- filesystem-based storage for downloaded media and exported backups

Suggested data domains:

- settings
- saved media
- history
- watch progress
- downloads
- subtitle attachments
- provider caches

### Backups and Restore

Backup behavior should be defined around explicit domain exports instead of raw key dumping.

Target approach:

- Rust produces backup payloads
- backup schema is versioned
- restore performs validation and migration

### Updates

Updates should remain desktop-native and be coordinated by Rust.

The UI should only show:

- available update state
- progress
- success/failure actions

## Initial Suggested Directory Direction

Frontend:

```text
src/
  app/
  components/
  features/
    library/
    search/
    player/
    downloads/
    settings/
    setup/
  lib/
    tauri/
  styles/
  types/
```

Backend:

```text
src-tauri/src/
  commands/
  services/
    metadata/
    search/
    library/
    downloads/
    subtitles/
    playback/
    settings/
    secrets/
    backups/
    updates/
  models/
  state/
  lib.rs
  main.rs
```

## Architecture Differences From Streambert

Compared with `/home/ankit/projects/opensource/streambert`, Ether should move toward:

- less renderer-owned business logic
- fewer oversized files with mixed responsibilities
- stronger typed contracts
- more structured persistence
- more explicit service boundaries
- reduced dependence on ad hoc localStorage state


## Related Docs

- `docs/MIGRATION.md`
  Migration strategy from Streambert to Ether.
