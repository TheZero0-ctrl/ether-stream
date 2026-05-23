# Migration Guide

## Goal

Ether is being built to recreate and improve the app currently located at:

- `/home/ankit/projects/opensource/streambert`

This is not just a framework swap from Electron to Tauri. It is an architectural rewrite.

## Migration Intent

We want to preserve the important product capabilities from Streambert:

- TMDB-powered discovery and metadata
- AniList-powered anime enrichment
- desktop-first playback workflows
- downloads and subtitle handling
- local library, history, and progress
- backups and update handling

We also want to improve:

- separation of concerns
- Rust ownership of core logic
- typed contracts between UI and backend
- persistence design
- maintainability of feature modules

## Source Project Reference

The current implementation to study and port from is:

- `/home/ankit/projects/opensource/streambert`

Useful source docs already written there:

- `/home/ankit/projects/opensource/streambert/local-docs/README.md`
- `/home/ankit/projects/opensource/streambert/local-docs/architecture.md`

## Migration Principles

### 1. Do Not Port Renderer Logic Blindly

Much of Streambert's important logic lives in React/Electron renderer code. Ether should not reproduce that pattern.

Preferred rule:

- if it touches persistence, downloads, subtitles, source resolution, or platform access, move it into Rust

### 2. Port Features By Domain, Not By File Name

Bad migration pattern:

- copy `MoviePage` and `TVPage` behavior directly and keep all logic in UI

Better migration pattern:

1. define the domain model
2. move provider/service logic into Rust
3. expose a typed command/event interface
4. rebuild the UI on top of that contract

### 3. Preserve Behavior First, Then Improve Internals

For risky flows like playback, downloads, and subtitles:

- first preserve user-facing behavior
- then improve implementation quality

### 4. Prefer Explicit Data Models

Streambert relies heavily on loosely structured localStorage state. Ether should move toward explicit models and versioned persistence.

## Recommended Migration Order

### Phase 1. Foundation

Build first:

- app shell
- command/event conventions
- typed frontend models
- error/result patterns
- settings and secret storage

### Phase 2. Persistence

Set up:

- SQLite schema via sqlx migrations (`src-tauri/migrations/`)
- repositories for settings, library, progress, downloads
- import/export schema shape

### Phase 3. Metadata Layer

Port into Rust:

- TMDB client
- AniList client
- basic caching strategy
- normalized media models

Target outcome:

- home feed
- search
- movie/show detail loading

### Phase 4. Library and Progress

Port:

- saved items
- history
- watched markers
- progress tracking

This provides a core product loop before harder media flows are finished.

### Phase 5. Downloads

Port and redesign:

- download registry
- queue management
- execution strategy
- progress events
- file validation and local discovery

### Phase 6. Subtitles

Port:

- provider integration
- search
- download
- extraction
- attachment logic

### Phase 7. Playback

This is the highest-risk area.

Port carefully:

- source selection
- player session behavior
- remote stream handling
- subtitle discovery/interception
- anime-specific source resolution

This phase may need temporary compromises while the architecture settles.

### Phase 8. Backups, Updates, Polish

Finish:

- backup/export/import
- update flow
- native window refinements
- onboarding/setup improvements

## Suggested Streambert-to-Ether Mapping

### Streambert Electron Main / IPC

Source:

- `index.js`
- `src/ipc/*.js`

Ether target:

- `src-tauri/src/services/*`
- `src-tauri/src/commands/*`

### Streambert Renderer App Shell

Source:

- `src/App.jsx`

Ether target:

- frontend app shell and feature-level route/state composition

### Streambert Utilities

Source:

- `src/utils/api.js`
- `src/utils/storage.js`
- `src/utils/backup.js`
- `src/utils/aniSkip.js`

Ether target:

- Rust services for provider and platform logic
- typed frontend helpers only where presentation-specific

### Streambert Pages

Source:

- `src/pages/*.jsx`

Ether target:

- frontend feature modules with less embedded business logic

## Data Migration Thoughts

Ether should eventually support importing meaningful user data from Streambert.

Possible import domains:

- saved library
- history
- watch progress
- watched flags
- settings that still make sense in Ether

Data sources in Streambert currently include:

- localStorage keys
- `downloads.json`
- secure-store values where compatible and appropriate

This import work should be explicit and versioned. Do not rely on raw storage copying.

## Risks

Main migration risks:

- recreating too much business logic in React
- trying to solve playback first
- preserving every implementation detail instead of preserving product value
- tying Ether too closely to Streambert's current storage layout

## Definition of Success

The migration is successful when Ether can deliver the core product experience of Streambert while having:

- cleaner service boundaries
- Rust-owned core logic
- typed frontend/backend contracts
- more maintainable persistence and state flows
- a better foundation for future features
