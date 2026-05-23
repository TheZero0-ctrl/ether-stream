# Anime Overview

## Goal

Anime is a first-class feature in Ether.

It should support both:

- anime movies
- anime series

The anime experience must recreate the useful behavior from Streambert while improving correctness, maintainability, and backend ownership.

## What Streambert Does Today

In Streambert, anime is not implemented as a separate backend feature. It is mostly a conditional branch inside movie and TV page logic.

Current behavior:

- anime is detected heuristically from TMDB metadata
- anime metadata is enriched with AniList
- anime playback defaults to `AllManga`
- TV anime seasons may be reinterpreted using AniList sequel relations
- intro/outro skipping is powered by AniSkip
- sub/dub switching is only implemented for the anime source path

## Ether Target

Ether should treat anime as a dedicated domain implemented primarily in Rust.

The React frontend should not own anime business logic beyond view state and user interaction.

## User-Facing Requirements

### Discovery

Users should be able to:

- see anime correctly labeled in discovery UIs
- search anime alongside other media
- open anime detail pages without needing to know whether TMDB or AniList is the richer source

### Metadata

Anime detail pages should show:

- canonical title and alternate titles
- poster and backdrop where available
- overview/description
- genres
- score
- season and episode structure
- release status where possible

### Playback

Users should be able to:

- play anime episodes and movies
- switch between sub and dub when available
- resume from saved progress
- open pop-out playback if supported by the chosen playback strategy
- see usable error states when a source cannot be resolved

### Skip Timing

For supported anime playback:

- fetch intro/outro timing data
- allow `off`, `auto`, and `manual` skip modes
- keep skip behavior source-aware rather than globally assuming all anime supports it

### Downloads and Subtitles

Anime should participate in the same desktop workflow as the rest of the product:

- download playable media
- attach subtitle files
- resume local playback later

## Important Design Rule

Anime should not be implemented as:

- a pile of special cases inside page components
- a frontend-only season remapping trick
- a title-search-only source workflow with no canonical backend model

Anime should instead be implemented as:

- a canonical media subtype
- a dedicated metadata enrichment path
- a dedicated episode mapping layer
- a dedicated playback/source resolution workflow

## Scope

This feature area includes:

- anime detection/classification
- AniList enrichment
- anime season/episode modeling
- source resolution for anime playback
- sub/dub selection
- AniSkip integration
- subtitle integration
- download integration

It does not require anime to have an entirely separate frontend application area. It can still reuse common library, details, and player UI patterns.
