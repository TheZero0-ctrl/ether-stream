# Ether

Initial desktop scaffold for rebuilding Streambert with `Tauri + Rust + React + TypeScript`.

## Stack

- Tauri 2
- Rust
- React
- TypeScript
- Vite

## Development

Install dependencies:

```bash
pnpm install
```

Run the desktop app in development:

```bash
pnpm tauri dev
```

Run the frontend only:

```bash
pnpm dev
```

Build the frontend:

```bash
pnpm build
```

Check the Rust side:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Current Scaffold

- `src/`
  React UI shell and typed Tauri bridge helpers.
- `src-tauri/`
  Rust commands and Tauri application setup.

## Suggested Next Steps

1. Add a real persistence layer for settings, library state, and downloads.
2. Create Rust service modules for TMDB, AniList, subtitles, and downloads.
3. Define typed frontend domain models for media, search results, and playback state.
4. Split the UI into feature areas like library, search, player, settings, and downloads.

## Recommended IDE Setup

- VS Code
- Tauri extension
- rust-analyzer
