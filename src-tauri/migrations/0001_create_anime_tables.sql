CREATE TABLE IF NOT EXISTS anime_identity_cache (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  media_kind TEXT NOT NULL,
  tmdb_id INTEGER,
  anilist_id INTEGER,
  mal_id INTEGER,
  canonical_title TEXT NOT NULL,
  romaji_title TEXT,
  english_title TEXT,
  native_title TEXT,
  title_aliases_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(tmdb_id, anilist_id, mal_id)
);

CREATE TABLE IF NOT EXISTS anilist_metadata_cache (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  anilist_id INTEGER NOT NULL UNIQUE,
  mal_id INTEGER,
  payload_json TEXT NOT NULL,
  fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS anime_episode_mapping_cache (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  canonical_episode_key TEXT NOT NULL UNIQUE,
  identity_key TEXT NOT NULL,
  season_number INTEGER,
  episode_number INTEGER,
  payload_json TEXT NOT NULL,
  fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS anime_progress (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  canonical_episode_key TEXT NOT NULL UNIQUE,
  identity_key TEXT NOT NULL,
  progress_seconds REAL NOT NULL DEFAULT 0,
  duration_seconds REAL,
  watched_completed INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS anime_skip_timings_cache (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mal_id INTEGER NOT NULL,
  episode_number INTEGER NOT NULL,
  segments_json TEXT NOT NULL,
  fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at TEXT NOT NULL,
  UNIQUE(mal_id, episode_number)
);

CREATE TABLE IF NOT EXISTS anime_translation_preferences (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  identity_key TEXT NOT NULL UNIQUE,
  translation_mode TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS anime_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  default_translation_mode TEXT NOT NULL,
  intro_skip_mode TEXT NOT NULL,
  preferred_subtitle_language TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
