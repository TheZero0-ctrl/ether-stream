# Anime Naming Conventions

This document defines naming conventions for anime commands, events, and model types.

## Commands (Rust + Tauri)

- Use `snake_case` prefixed with `anime_`.
- Commands should follow `anime_<verb>_<object>` where possible.

Current command contract:

- `anime_get_details`
- `anime_get_episode_list`
- `anime_resolve_playback`
- `anime_get_skip_timings`
- `anime_set_translation_mode`
- `anime_prepare_download`

## Events (Rust emits, frontend subscribes)

- Use `kebab-case` prefixed with `anime-`.
- Events should follow `anime-<domain>-<state>`.

Current event contract:

- `anime-playback-ready`
- `anime-playback-failed`
- `anime-progress-updated`
- `anime-skip-segment-active`

## Rust Type Names

- Use `PascalCase` for structs/enums/traits.
- Prefix anime domain models with `Anime` when shared context can be ambiguous.
- Use explicit error variant names (example: `TranslationUnavailable`).

## TypeScript Type Names

- Use `PascalCase` for exported types.
- Mirror Rust payload model names where practical.
- Event constant maps should be `UPPER_SNAKE_CASE` when shared globally or
  `camelCase` object keys when grouped by feature.

## File and Module Names

- Rust modules/files: `snake_case`.
- TypeScript feature files: `kebab-case` or `index.ts` exports.
- Keep provider-specific names out of shared UI component filenames.
