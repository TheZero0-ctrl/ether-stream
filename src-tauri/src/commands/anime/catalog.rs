use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};

use super::AnimeCatalogItem;

pub(super) async fn fetch_anilist_json(
    payload: serde_json::Value,
) -> Result<serde_json::Value, AnimeCommandError> {
    let response = reqwest::Client::new()
        .post("https://graphql.anilist.co")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "anilist request failed".to_string(),
            context: Some(err.to_string()),
        })?;

    if !response.status().is_success() {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "anilist returned non-success status".to_string(),
            context: Some(response.status().to_string()),
        });
    }

    response.json().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::AnilistMatchMissing,
        message: "anilist payload parse failed".to_string(),
        context: Some(err.to_string()),
    })
}

pub(super) fn parse_catalog_items(value: serde_json::Value) -> Vec<AnimeCatalogItem> {
    value
        .get("data")
        .and_then(|v| v.get("Page"))
        .and_then(|v| v.get("media"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let anilist_id = item.get("id")?.as_i64()?;
            let title = item
                .get("title")
                .and_then(|v| v.get("english"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    item.get("title")
                        .and_then(|v| v.get("romaji"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Unknown".to_string());

            Some(AnimeCatalogItem {
                anilist_id,
                mal_id: item.get("idMal").and_then(|v| v.as_i64()),
                title,
                poster_url: item
                    .get("coverImage")
                    .and_then(|v| v.get("large"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                year: item.get("seasonYear").and_then(|v| v.as_i64()).map(|v| v as i32),
                episodes: item.get("episodes").and_then(|v| v.as_i64()).map(|v| v as i32),
            })
        })
        .collect()
}
