use sqlx::{Row, SqlitePool};

use super::models::{AnimeDetails, AnimeIdentity, AnimeMediaKind, AnimeSkipTimings, AnimeTranslationMode};

#[derive(Clone)]
pub struct AnimeSqlxRepository {
    pool: SqlitePool,
}

impl AnimeSqlxRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_identity(
        &self,
        _identity_key: &str,
        identity: &AnimeIdentity,
    ) -> Result<(), sqlx::Error> {
        let aliases_json = serde_json::to_string(&identity.title_aliases)
            .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
        let media_kind = match identity.media_kind {
            AnimeMediaKind::AnimeSeries => "animeSeries",
            AnimeMediaKind::AnimeMovie => "animeMovie",
        };

        sqlx::query(
            "INSERT INTO anime_identity_cache (
                media_kind, tmdb_id, anilist_id, mal_id, canonical_title,
                romaji_title, english_title, native_title, title_aliases_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(tmdb_id, anilist_id, mal_id) DO UPDATE SET
                media_kind = excluded.media_kind,
                canonical_title = excluded.canonical_title,
                romaji_title = excluded.romaji_title,
                english_title = excluded.english_title,
                native_title = excluded.native_title,
                title_aliases_json = excluded.title_aliases_json,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(media_kind)
        .bind(identity.tmdb_id)
        .bind(identity.anilist_id)
        .bind(identity.mal_id)
        .bind(&identity.canonical_title)
        .bind(&identity.romaji_title)
        .bind(&identity.english_title)
        .bind(&identity.native_title)
        .bind(aliases_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_translation_preference(
        &self,
        identity_key: &str,
        mode: AnimeTranslationMode,
    ) -> Result<(), sqlx::Error> {
        let mode_value = match mode {
            AnimeTranslationMode::Sub => "sub",
            AnimeTranslationMode::Dub => "dub",
        };

        sqlx::query(
            "INSERT INTO anime_translation_preferences (identity_key, translation_mode)
            VALUES (?, ?)
            ON CONFLICT(identity_key) DO UPDATE SET
                translation_mode = excluded.translation_mode,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(identity_key)
        .bind(mode_value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct AnimeSqlxCacheRepository {
    pool: SqlitePool,
}

impl AnimeSqlxCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn put_anilist_metadata(
        &self,
        anilist_id: i64,
        mal_id: Option<i64>,
        details: &AnimeDetails,
        expires_at: &str,
    ) -> Result<(), sqlx::Error> {
        let payload_json = serde_json::to_string(details)
            .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;

        sqlx::query(
            "INSERT INTO anilist_metadata_cache (anilist_id, mal_id, payload_json, expires_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(anilist_id) DO UPDATE SET
                mal_id = excluded.mal_id,
                payload_json = excluded.payload_json,
                fetched_at = CURRENT_TIMESTAMP,
                expires_at = excluded.expires_at",
        )
        .bind(anilist_id)
        .bind(mal_id)
        .bind(payload_json)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_anilist_metadata(&self, anilist_id: i64) -> Result<Option<AnimeDetails>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT payload_json
             FROM anilist_metadata_cache
             WHERE anilist_id = ? AND expires_at > CURRENT_TIMESTAMP",
        )
        .bind(anilist_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let payload: String = row.try_get("payload_json")?;
                let details = serde_json::from_str::<AnimeDetails>(&payload)
                    .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
                Ok(Some(details))
            }
            None => Ok(None),
        }
    }

    pub async fn put_skip_timings(
        &self,
        mal_id: i64,
        episode_number: i32,
        timings: &AnimeSkipTimings,
        expires_at: &str,
    ) -> Result<(), sqlx::Error> {
        let segments_json = serde_json::to_string(&timings.segments)
            .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;

        sqlx::query(
            "INSERT INTO anime_skip_timings_cache (mal_id, episode_number, segments_json, expires_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(mal_id, episode_number) DO UPDATE SET
               segments_json = excluded.segments_json,
               fetched_at = CURRENT_TIMESTAMP,
               expires_at = excluded.expires_at",
        )
        .bind(mal_id)
        .bind(episode_number)
        .bind(segments_json)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_skip_timings(
        &self,
        mal_id: i64,
        episode_number: i32,
    ) -> Result<Option<AnimeSkipTimings>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT segments_json
             FROM anime_skip_timings_cache
             WHERE mal_id = ? AND episode_number = ? AND expires_at > CURRENT_TIMESTAMP",
        )
        .bind(mal_id)
        .bind(episode_number)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let segments_json: String = row.try_get("segments_json")?;
                let segments = serde_json::from_str(&segments_json)
                    .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;

                Ok(Some(AnimeSkipTimings {
                    mal_id,
                    episode_number,
                    segments,
                }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct AnimeSqlxProgressRepository {
    pool: SqlitePool,
}

impl AnimeSqlxProgressRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_progress(
        &self,
        canonical_episode_key: &str,
        identity_key: &str,
        progress_seconds: f64,
        duration_seconds: Option<f64>,
        watched_completed: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO anime_progress (
                canonical_episode_key, identity_key, progress_seconds, duration_seconds, watched_completed
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(canonical_episode_key) DO UPDATE SET
                progress_seconds = excluded.progress_seconds,
                duration_seconds = excluded.duration_seconds,
                watched_completed = excluded.watched_completed,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(canonical_episode_key)
        .bind(identity_key)
        .bind(progress_seconds)
        .bind(duration_seconds)
        .bind(if watched_completed { 1 } else { 0 })
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_progress(
        &self,
        canonical_episode_key: &str,
    ) -> Result<Option<(f64, Option<f64>, bool)>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT progress_seconds, duration_seconds, watched_completed
             FROM anime_progress
             WHERE canonical_episode_key = ?",
        )
        .bind(canonical_episode_key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let progress: f64 = row.try_get("progress_seconds")?;
                let duration: Option<f64> = row.try_get("duration_seconds")?;
                let watched: i64 = row.try_get("watched_completed")?;
                Ok(Some((progress, duration, watched != 0)))
            }
            None => Ok(None),
        }
    }

    pub async fn get_latest_progress_for_identity(
        &self,
        identity_key: &str,
    ) -> Result<Option<(String, f64, Option<f64>, bool)>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT canonical_episode_key, progress_seconds, duration_seconds, watched_completed
             FROM anime_progress
             WHERE identity_key = ?
             ORDER BY updated_at DESC, id DESC
             LIMIT 1",
        )
        .bind(identity_key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let canonical_episode_key: String = row.try_get("canonical_episode_key")?;
                let progress: f64 = row.try_get("progress_seconds")?;
                let duration: Option<f64> = row.try_get("duration_seconds")?;
                let watched: i64 = row.try_get("watched_completed")?;
                Ok(Some((canonical_episode_key, progress, duration, watched != 0)))
            }
            None => Ok(None),
        }
    }

    pub async fn set_last_episode(
        &self,
        identity_key: &str,
        season_number: Option<i32>,
        episode_number: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO anime_resume_state (identity_key, season_number, episode_number)
             VALUES (?, ?, ?)
             ON CONFLICT(identity_key) DO UPDATE SET
                season_number = excluded.season_number,
                episode_number = excluded.episode_number,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(identity_key)
        .bind(season_number)
        .bind(episode_number)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_last_episode(
        &self,
        identity_key: &str,
    ) -> Result<Option<(Option<i32>, Option<i32>)>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT season_number, episode_number
             FROM anime_resume_state
             WHERE identity_key = ?",
        )
        .bind(identity_key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let season: Option<i32> = row.try_get("season_number")?;
                let episode: Option<i32> = row.try_get("episode_number")?;
                Ok(Some((season, episode)))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_progress_repo() -> AnimeSqlxProgressRepository {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");

        sqlx::query(
            "CREATE TABLE anime_progress (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                canonical_episode_key TEXT NOT NULL UNIQUE,
                identity_key TEXT NOT NULL,
                progress_seconds REAL NOT NULL DEFAULT 0,
                duration_seconds REAL,
                watched_completed INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .expect("anime_progress table should be created");

        sqlx::query(
            "CREATE TABLE anime_resume_state (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                identity_key TEXT NOT NULL UNIQUE,
                season_number INTEGER,
                episode_number INTEGER,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .expect("anime_resume_state table should be created");

        AnimeSqlxProgressRepository::new(pool)
    }

    #[test]
    fn upsert_and_get_progress_round_trip() {
        tauri::async_runtime::block_on(async {
            let repo = setup_progress_repo().await;

            repo.upsert_progress("ep-key-1", "id-key-1", 123.0, Some(1440.0), false)
                .await
                .expect("progress should insert");

            let row = repo
                .get_progress("ep-key-1")
                .await
                .expect("progress read should succeed")
                .expect("progress row should exist");

            assert_eq!(row.0, 123.0);
            assert_eq!(row.1, Some(1440.0));
            assert!(!row.2);
        });
    }

    #[test]
    fn latest_progress_for_identity_returns_most_recent_row() {
        tauri::async_runtime::block_on(async {
            let repo = setup_progress_repo().await;

            repo.upsert_progress("ep-key-1", "id-key-1", 10.0, Some(100.0), false)
                .await
                .expect("first row should insert");
            repo.upsert_progress("ep-key-2", "id-key-1", 88.0, Some(100.0), true)
                .await
                .expect("second row should insert");

            let latest = repo
                .get_latest_progress_for_identity("id-key-1")
                .await
                .expect("latest progress read should succeed")
                .expect("latest row should exist");

            assert_eq!(latest.0, "ep-key-2");
            assert_eq!(latest.1, 88.0);
            assert!(latest.3);
        });
    }

    #[test]
    fn set_and_get_last_episode_round_trip() {
        tauri::async_runtime::block_on(async {
            let repo = setup_progress_repo().await;

            repo.set_last_episode("id-key-1", Some(1), Some(7))
                .await
                .expect("last episode should save");

            let row = repo
                .get_last_episode("id-key-1")
                .await
                .expect("last episode read should succeed")
                .expect("last episode row should exist");

            assert_eq!(row.0, Some(1));
            assert_eq!(row.1, Some(7));
        });
    }

    #[test]
    fn set_last_episode_upserts_existing_identity() {
        tauri::async_runtime::block_on(async {
            let repo = setup_progress_repo().await;

            repo.set_last_episode("id-key-1", Some(1), Some(3))
                .await
                .expect("initial value should save");
            repo.set_last_episode("id-key-1", Some(1), Some(9))
                .await
                .expect("updated value should save");

            let row = repo
                .get_last_episode("id-key-1")
                .await
                .expect("read should succeed")
                .expect("row should exist");

            assert_eq!(row.1, Some(9));
        });
    }
}
