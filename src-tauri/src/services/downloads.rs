use std::sync::{Arc, Mutex, OnceLock};
use std::process::Command;

use sqlx::{Row, SqlitePool};
use tokio::sync::Semaphore;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::models::{AnimeDownloadPayload, AnimePlaybackKind};

static DOWNLOAD_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static CANCELLED_IDS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    pub id: String,
    pub payload: AnimeDownloadPayload,
    pub status: String,
    pub bytes_downloaded: i64,
    pub total_bytes: i64,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

pub async fn enqueue(pool: &SqlitePool, id: &str, payload: &AnimeDownloadPayload) -> Result<(), AnimeCommandError> {
    let payload_json = serde_json::to_string(payload).map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to serialize download payload".to_string(),
        context: Some(err.to_string()),
    })?;

    sqlx::query(
        r#"INSERT OR REPLACE INTO downloads (id, payload_json, status, bytes_downloaded, total_bytes, output_path, error)
           VALUES (?1, ?2, 'queued', 0, 0, NULL, NULL)"#,
    )
    .bind(id)
    .bind(payload_json)
    .execute(pool)
    .await
    .map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to enqueue download".to_string(),
        context: Some(err.to_string()),
    })?;

    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<DownloadRecord>, AnimeCommandError> {
    let rows = sqlx::query(
        r#"SELECT id, payload_json, status, bytes_downloaded, total_bytes, output_path, error
           FROM downloads
           ORDER BY created_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to list downloads".to_string(),
        context: Some(err.to_string()),
    })?;

    let mut records = Vec::new();
    for row in rows {
        let payload_json: String = row.get("payload_json");
        let payload: AnimeDownloadPayload = serde_json::from_str(&payload_json).map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to parse download payload".to_string(),
            context: Some(err.to_string()),
        })?;

        records.push(DownloadRecord {
            id: row.get("id"),
            payload,
            status: row.get("status"),
            bytes_downloaded: row.get::<i64, _>("bytes_downloaded"),
            total_bytes: row.get::<i64, _>("total_bytes"),
            output_path: row.get("output_path"),
            error: row.get("error"),
        });
    }
    Ok(records)
}

pub async fn cancel(pool: &SqlitePool, id: &str) -> Result<(), AnimeCommandError> {
    if let Ok(mut ids) = cancelled_ids().lock() {
        if !ids.contains(&id.to_string()) {
            ids.push(id.to_string());
        }
    }
    update_status(pool, id, "cancelled", None, None, None).await
}

pub async fn remove(pool: &SqlitePool, id: &str) -> Result<Option<AnimeDownloadPayload>, AnimeCommandError> {
    let row = sqlx::query("SELECT payload_json FROM downloads WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to load download payload".to_string(),
            context: Some(err.to_string()),
        })?;

    let payload = if let Some(row) = row {
        let payload_json: String = row.get("payload_json");
        Some(serde_json::from_str(&payload_json).map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to parse download payload".to_string(),
            context: Some(err.to_string()),
        })?)
    } else {
        None
    };

    sqlx::query("DELETE FROM downloads WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed to delete download record".to_string(),
            context: Some(err.to_string()),
        })?;

    Ok(payload)
}

pub async fn start_worker(pool: SqlitePool, max_parallel: usize) {
    let semaphore = DOWNLOAD_SEMAPHORE.get_or_init(|| Arc::new(Semaphore::new(max_parallel)));
    loop {
        let queued = sqlx::query(
            r#"SELECT id, payload_json FROM downloads
               WHERE status = 'queued'
               ORDER BY created_at ASC"#,
        )
        .fetch_all(&pool)
        .await;

        if let Ok(rows) = queued {
            for row in rows {
                let permit = semaphore.clone().acquire_owned().await;
                if permit.is_err() {
                    continue;
                }
                let permit = permit.unwrap();
                let pool = pool.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Ok(id) = row.try_get::<String, _>("id") {
                        let payload_json: String = row.try_get("payload_json").unwrap_or_default();
                        let payload: Result<AnimeDownloadPayload, _> = serde_json::from_str(&payload_json);
                        if let Ok(payload) = payload {
                            let _ = update_status(&pool, &id, "downloading", None, None, None).await;
                            let output_dir = build_download_output_dir(&payload);
                            let _ = std::fs::create_dir_all(&output_dir);
                            let output_path = output_dir.join(&payload.file_name);
                            let tmp_path = output_dir.join(format!("{}.part", payload.file_name));
                            if is_cancelled(&id) {
                                let _ = update_status(&pool, &id, "cancelled", None, None, None).await;
                                return;
                            }
                            let client = reqwest::Client::builder()
                                .connect_timeout(std::time::Duration::from_secs(10))
                                .timeout(std::time::Duration::from_secs(60 * 30))
                                .build();
                            if let Ok(client) = client {
                                let is_hls = payload.playback_kind == AnimePlaybackKind::Hls
                                    || payload.playback_url.contains(".m3u8");
                                let mut result = if is_hls {
                                    download_hls_to_temp_file(&payload, &id, &tmp_path, &pool).await
                                } else {
                                    download_to_temp_file(&client, &payload, &id, &tmp_path, &pool).await
                                };
                                if !is_hls {
                                    if let Ok(bytes) = result {
                                        if bytes == 0 {
                                            result = download_hls_to_temp_file(&payload, &id, &tmp_path, &pool).await;
                                        } else {
                                            result = Ok(bytes);
                                        }
                                    }
                                }
                                if !is_hls {
                                    if let Err(err) = &result {
                                        if err.message == "hls manifest detected" {
                                            result = download_hls_to_temp_file(&payload, &id, &tmp_path, &pool).await;
                                        }
                                    }
                                }
                                match result {
                                    Ok(bytes) => {
                                        if is_cancelled(&id) {
                                            let _ = std::fs::remove_file(&tmp_path);
                                            let _ = update_status(&pool, &id, "cancelled", None, None, None).await;
                                            return;
                                        }
                                        let _ = std::fs::rename(&tmp_path, &output_path);
                                        let _ = update_status(
                                            &pool,
                                            &id,
                                            "completed",
                                            Some(bytes as i64),
                                            Some(output_path.display().to_string()),
                                            None,
                                        )
                                        .await;
                                    }
                                    Err(err) => {
                                        let _ = std::fs::remove_file(&tmp_path);
                    let _ = update_status(
                        &pool,
                        &id,
                        "failed",
                        Some(0),
                        None,
                        Some(err.message),
                    )
                    .await;
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    }
}

fn build_download_output_dir(payload: &AnimeDownloadPayload) -> std::path::PathBuf {
    build_download_output_dir_from_parts(&payload.media_name, payload.season_number)
}

fn build_download_output_dir_from_parts(media_name: &str, season_number: Option<i32>) -> std::path::PathBuf {
    let anime_name = sanitize_path_segment(media_name);
    let season = season_number
        .map(|number| format!("Season_{number:02}"))
        .unwrap_or_else(|| "Season_00".to_string());
    default_download_dir().join(anime_name).join(season)
}

fn default_download_dir() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join("Downloads").join("Ether");
    }
    std::env::temp_dir().join("EtherDownloads")
}

fn sanitize_path_segment(input: &str) -> String {
    let cleaned = input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if cleaned.is_empty() {
        "Unknown_Anime".to_string()
    } else {
        cleaned
    }
}

async fn download_to_temp_file(
    client: &reqwest::Client,
    payload: &AnimeDownloadPayload,
    download_id: &str,
    tmp_path: &std::path::PathBuf,
    pool: &SqlitePool,
) -> Result<u64, AnimeCommandError> {
    let mut request_builder = client.get(&payload.playback_url);
    for (key, value) in &payload.request_headers {
        request_builder = request_builder.header(key, value);
    }
    let response = request_builder.send().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "failed to download anime media".to_string(),
        context: Some(err.to_string()),
    })?;
    if !response.status().is_success() {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "download request failed".to_string(),
            context: Some(format!("status={}", response.status())),
        });
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    if content_type.contains("application/vnd.apple.mpegurl")
        || content_type.contains("application/x-mpegurl")
        || payload.playback_url.contains(".m3u8")
    {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "hls manifest detected".to_string(),
            context: Some(content_type),
        });
    }

    let total_bytes = response.content_length().unwrap_or(0) as i64;
    let _ = set_total_bytes(pool, download_id, total_bytes).await;

    let mut stream = response.bytes_stream();
    let first = match stream.next().await {
        Some(chunk) => chunk.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "failed while reading download stream".to_string(),
            context: Some(err.to_string()),
        })?,
        None => {
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::ProviderSearchFailed,
                message: "download returned empty response".to_string(),
                context: Some("stream empty".to_string()),
            });
        }
    };

    let first_preview = String::from_utf8_lossy(&first);
    let first_trimmed = first_preview.trim_start();
    if content_type.contains("text/html") || content_type.contains("application/xhtml") {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "download source is not a media stream".to_string(),
            context: Some(content_type),
        });
    }
    if first_trimmed.starts_with("#EXTM3U")
        || first_trimmed.contains("#EXT-X-STREAM-INF")
        || first_trimmed.contains("#EXTINF")
    {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "hls manifest detected".to_string(),
            context: Some("manifest header".to_string()),
        });
    }

    let mut file = tokio::fs::File::create(tmp_path).await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to create temporary download file".to_string(),
        context: Some(err.to_string()),
    })?;
    let mut total = 0_u64;
    let mut last_reported = 0_u64;
    total += first.len() as u64;
    file.write_all(&first).await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed writing download chunk".to_string(),
        context: Some(err.to_string()),
    })?;
    while let Some(chunk) = stream.next().await {
        if is_cancelled(download_id) {
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::ProviderSearchFailed,
                message: "download cancelled".to_string(),
                context: None,
            });
        }
        let chunk = chunk.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "failed while reading download stream".to_string(),
            context: Some(err.to_string()),
        })?;
        total += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "failed writing download chunk".to_string(),
            context: Some(err.to_string()),
        })?;

        // Throttle progress writes to roughly every ~2MB.
        if total - last_reported >= 2 * 1024 * 1024 {
            last_reported = total;
            let _ = set_bytes_downloaded(pool, download_id, total as i64).await;
        }
    }
    file.flush().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to flush download file".to_string(),
        context: Some(err.to_string()),
    })?;
    if total == 0 {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "download returned empty response".to_string(),
            context: Some("bytes=0".to_string()),
        });
    }
    let _ = set_bytes_downloaded(pool, download_id, total as i64).await;
    Ok(total)
}

async fn download_hls_to_temp_file(
    payload: &AnimeDownloadPayload,
    download_id: &str,
    tmp_path: &std::path::PathBuf,
    pool: &SqlitePool,
) -> Result<u64, AnimeCommandError> {
    let mut header_value = String::new();
    for (key, value) in &payload.request_headers {
        if key.eq_ignore_ascii_case("user-agent") {
            continue;
        }
        header_value.push_str(key);
        header_value.push_str(": ");
        header_value.push_str(value);
        header_value.push_str("\r\n");
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    cmd.arg("-hide_banner");
    cmd.arg("-loglevel").arg("error");
    if !header_value.is_empty() {
        cmd.arg("-headers").arg(header_value);
    }
    if let Some(referer) = &payload.referer {
        cmd.arg("-referer").arg(referer);
    }
    if let Some(user_agent) = payload
        .request_headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("user-agent"))
        .map(|(_, value)| value)
    {
        cmd.arg("-user_agent").arg(user_agent);
    }
    cmd.arg("-allowed_extensions").arg("ALL");
    cmd.arg("-i").arg(&payload.playback_url);
    cmd.arg("-c").arg("copy");
    cmd.arg(tmp_path);

    let mut child = cmd.spawn().map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "failed to start ffmpeg download".to_string(),
        context: Some(err.to_string()),
    })?;

    loop {
        if is_cancelled(download_id) {
            let _ = child.kill();
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::ProviderSearchFailed,
                message: "download cancelled".to_string(),
                context: None,
            });
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(AnimeCommandError {
                        category: AnimeErrorCategory::ProviderSearchFailed,
                        message: "ffmpeg download failed".to_string(),
                        context: status.code().map(|code| format!("code={code}")).or(Some("unknown".to_string())),
                    });
                }
                break;
            }
            Ok(None) => {
                if let Ok(metadata) = tokio::fs::metadata(tmp_path).await {
                    let size = metadata.len() as i64;
                    let _ = set_bytes_downloaded(pool, download_id, size).await;
                }
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            }
            Err(err) => {
                return Err(AnimeCommandError {
                    category: AnimeErrorCategory::ProviderSearchFailed,
                    message: "ffmpeg download failed".to_string(),
                    context: Some(err.to_string()),
                });
            }
        }
    }

    let final_size = tokio::fs::metadata(tmp_path)
        .await
        .map(|meta| meta.len())
        .unwrap_or(0);
    if final_size == 0 {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "ffmpeg produced empty file".to_string(),
            context: Some("bytes=0".to_string()),
        });
    }
    let _ = set_bytes_downloaded(pool, download_id, final_size as i64).await;
    Ok(final_size)
}

async fn set_total_bytes(pool: &SqlitePool, id: &str, total_bytes: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE downloads SET total_bytes = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?1")
        .bind(id)
        .bind(total_bytes)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn set_bytes_downloaded(pool: &SqlitePool, id: &str, bytes: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE downloads SET bytes_downloaded = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?1")
        .bind(id)
        .bind(bytes)
        .execute(pool)
        .await
        .map(|_| ())
}

async fn update_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    bytes_downloaded: Option<i64>,
    output_path: Option<String>,
    error: Option<String>,
) -> Result<(), AnimeCommandError> {
    sqlx::query(
        r#"UPDATE downloads
           SET status = ?2,
               bytes_downloaded = COALESCE(?3, bytes_downloaded),
               output_path = COALESCE(?4, output_path),
               error = ?5,
               updated_at = CURRENT_TIMESTAMP
           WHERE id = ?1"#,
    )
    .bind(id)
    .bind(status)
    .bind(bytes_downloaded)
    .bind(output_path)
    .bind(error)
    .execute(pool)
    .await
    .map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::PlayableSourceMissing,
        message: "failed to update download status".to_string(),
        context: Some(err.to_string()),
    })?;
    Ok(())
}

fn cancelled_ids() -> &'static Mutex<Vec<String>> {
    CANCELLED_IDS.get_or_init(|| Mutex::new(Vec::new()))
}

fn is_cancelled(id: &str) -> bool {
    cancelled_ids()
        .lock()
        .ok()
        .map(|list| list.contains(&id.to_string()))
        .unwrap_or(false)
}
