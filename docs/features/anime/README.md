# Anime Feature Docs

This directory documents how anime should be implemented in Ether.

Ether is trying to recreate and improve the anime functionality currently found in `/home/ankit/projects/opensource/streambert`.

These docs are implementation-oriented. They are intended to guide the Rust services, Tauri command/event contracts, and React screens needed for anime support.

## Documents

- `overview.md`
  Product behavior, goals, and scope.
- `architecture.md`
  Runtime responsibilities and service boundaries.
- `data-model.md`
  Canonical domain model for anime series, episodes, playback, and skip data.
- `provider-flow.md`
  Provider resolution flow from TMDB to AniList to stream resolver.
- `playback.md`
  Playback, progress, sub/dub, AniSkip, subtitles, and downloads interaction.
- `naming-conventions.md`
  Command, event, model, and module naming rules for anime contracts.
- `migration-notes.md`
  What exists in Streambert today, what to preserve, and what to redesign.

## Source Reference

Current anime implementation reference:

- `/home/ankit/projects/opensource/streambert/src/utils/api.js`
- `/home/ankit/projects/opensource/streambert/src/utils/aniSkip.js`
- `/home/ankit/projects/opensource/streambert/src/utils/episodeMappings.js`
- `/home/ankit/projects/opensource/streambert/src/pages/TVPage.jsx`
- `/home/ankit/projects/opensource/streambert/src/pages/MoviePage.jsx`
- `/home/ankit/projects/opensource/streambert/src/ipc/allmanga.js`
