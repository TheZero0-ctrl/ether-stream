# Anime Migration Notes

## Source Implementation

Current anime implementation lives across the following Streambert files:

- `/home/ankit/projects/opensource/streambert/src/utils/api.js`
- `/home/ankit/projects/opensource/streambert/src/utils/aniSkip.js`
- `/home/ankit/projects/opensource/streambert/src/utils/episodeMappings.js`
- `/home/ankit/projects/opensource/streambert/src/pages/TVPage.jsx`
- `/home/ankit/projects/opensource/streambert/src/pages/MoviePage.jsx`
- `/home/ankit/projects/opensource/streambert/src/ipc/allmanga.js`

## What To Preserve

These product behaviors matter and should survive the rewrite:

- anime gets richer metadata than plain TMDB content
- anime defaults to a dedicated source path
- TV anime can present better season structure than raw TMDB alone
- sub/dub switching exists in the player workflow
- AniSkip exists for supported anime playback
- anime downloads and subtitle flows integrate with the rest of the app

## What To Redesign

### 1. Detection

Current state:

- simple heuristic based on animation genre and Japanese language/origin

Ether direction:

- keep compatibility heuristic first
- wrap it in a real classifier service
- allow stronger classification later

### 2. AniList identity matching

Current state:

- title-search based
- substring validation on cache reuse

Ether direction:

- build a proper identity reconciliation layer
- keep title search as a fallback, not the core identity model

### 3. Season mapping

Current state:

- TV page slices TMDB season 1 episodes into AniList virtual seasons
- normal TV remapping and anime remapping are separate concepts

Ether direction:

- create one canonical backend episode mapping layer

### 4. Provider resolution

Current state:

- provider resolution includes hardcoded IDs, split-season exceptions, title sanitization, and search heuristics mixed together

Ether direction:

- isolate provider compatibility overrides
- make provider resolution a dedicated backend service with explicit steps and error types

### 5. Playback state ownership

Current state:

- page components own a lot of playback orchestration
- resolved media server state in main process is effectively singleton/global

Ether direction:

- move playback orchestration into backend services
- expose stable playback sessions to the frontend

### 6. AniSkip coupling

Current state:

- skip logic is tightly coupled to TV page behavior and the AllManga source path

Ether direction:

- make skip timing a reusable playback capability

## Mapping Streambert Concepts To Ether

### `isAnimeContent`

Streambert:

- `src/utils/api.js`

Ether target:

- `services/anime/classifier`

### `fetchAnilistData` and `buildAnilistSeasons`

Streambert:

- `src/utils/api.js`

Ether target:

- `services/anime/metadata`
- `services/anime/mapping`

### `resolve-allmanga`

Streambert:

- `src/ipc/allmanga.js`

Ether target:

- `services/anime/resolver`
- possibly shared playback proxy helpers in a general playback service

### AniSkip integration

Streambert:

- `src/utils/aniSkip.js`
- `src/pages/TVPage.jsx`

Ether target:

- `services/anime/skip`
- shared playback session integration

## Recommended Implementation Order

1. classifier
2. metadata enrichment
3. canonical season/episode mapping
4. resolver service
5. playback session model
6. AniSkip integration
7. download/subtitle preparation

## Rule For The Rewrite

Do not port anime by copying Streambert page logic into React.

Port anime by rebuilding the backend model first, then layering UI on top of it.
