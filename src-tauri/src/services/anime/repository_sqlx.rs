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
}
