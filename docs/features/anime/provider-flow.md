# Anime Provider Flow

## Goal

This document describes how Ether should move from a TMDB-discovered media item to a resolved anime playback source.

## Current Streambert Pattern

Streambert does the following:

1. detect anime from TMDB heuristics
2. fetch AniList metadata by title search
3. derive alternate season/title forms
4. search AllManga or AllAnime using candidate titles
5. resolve source URLs through provider-specific decoding logic

This works, but it is brittle and spread across renderer and IPC code.

## Ether Target Flow

### Phase 1. Entry

Input usually begins from a TMDB-origin item selected in search, home, or library.

Input payload should include:

- TMDB media id
- media kind
- title and alternate title fields if already known
- selected season/episode for series playback

### Phase 2. Classification

Backend checks whether the item belongs to the anime pipeline.

Initial rule can match Streambert compatibility:

- animation genre
- Japanese language or origin signal

Output:

- `is_anime: true|false`
- optional confidence and reasons

### Phase 3. Metadata Enrichment

If anime is true:

1. fetch AniList match
2. normalize title variants
3. capture `anilist_id` and `mal_id`
4. capture season structure if AniList provides a more useful representation

Output should be a normalized anime identity and details object.

### Phase 4. Episode Mapping

For series playback, backend resolves canonical season/episode information.

This step must determine:

- what season/episode the user should see
- what TMDB episode corresponds to that view
- what provider-facing episode identifier should be used

This replaces Streambert's page-level season slicing behavior.

### Phase 5. Provider Search Candidate Generation

Backend creates a candidate search list from normalized titles.

Possible inputs:

- canonical English title
- romaji title
- native title if needed
- sanitized variants
- season-specific title variant
- fallback original title from TMDB

This should be a backend concern only.

### Phase 6. Provider Resolution

For the selected provider, backend:

1. searches provider index
2. chooses best match
3. requests episode/movie source information
4. decodes or transforms provider payloads as needed
5. chooses best playable source based on priority rules

### Phase 7. Playback Result

The provider resolver returns a normalized playback result.

Example information:

- resolved provider name
- resolved media URL
- playback mode
- required referer
- subtitle candidates if known
- whether the result is suitable for download

## Resolver Requirements

### Supported fallback behavior

Resolver should support:

- multiple title candidates
- sub/dub fallback handling
- per-show hardcoded overrides as an isolated escape hatch
- multi-provider future expansion

### Hardcoded rules

If hardcoded rules are unavoidable, they should live in a dedicated compatibility layer, not inside the main resolver flow.

Examples from Streambert that should be isolated if carried forward:

- split season overrides
- hardcoded provider show IDs

## Error Model

Resolver should return structured errors, not just strings.

Suggested categories:

- `AnimeNotClassified`
- `AnilistMatchMissing`
- `SeasonMappingFailed`
- `ProviderSearchFailed`
- `ProviderEpisodeMissing`
- `PlayableSourceMissing`
- `TranslationUnavailable`

## Caching

Resolver should cache at appropriate layers:

- AniList metadata
- provider show lookup results
- provider episode resolution results when safe
- skip timing results

Caching should live in Rust, ideally with persistence support.

## Main Design Rule

React should request `resolve anime playback` and receive a normalized answer.

React should not generate provider title guesses or reconcile AniList/TMDB numbering itself.
