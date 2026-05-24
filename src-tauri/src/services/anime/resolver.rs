use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use aes::Aes256;
use base64::Engine as _;
use ctr::cipher::{KeyIvInit, StreamCipher};
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::errors::{AnimeCommandError, AnimeErrorCategory};
use super::models::{
    AnimeIdentity, AnimePlaybackKind, AnimePlaybackSource, AnimeProvider, AnimeTranslationMode,
    DiscoveredSubtitle,
};

#[derive(Clone)]
struct CachedProviderResolution {
    stream_url: String,
    expires_at: Instant,
}

static PROVIDER_CACHE: OnceLock<Mutex<HashMap<String, CachedProviderResolution>>> = OnceLock::new();
static SHOW_ID_CACHE: OnceLock<Mutex<HashMap<String, CachedProviderResolution>>> = OnceLock::new();
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

const NEGATIVE_CACHE_SENTINEL: &str = "__MISS__";
const SHOW_LOOKUP_TIMEOUT: Duration = Duration::from_secs(7);
const EPISODE_FETCH_TIMEOUT: Duration = Duration::from_secs(9);
const SOURCE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_SOURCE_ATTEMPTS: usize = 4;

fn provider_cache() -> &'static Mutex<HashMap<String, CachedProviderResolution>> {
    PROVIDER_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn show_id_cache() -> &'static Mutex<HashMap<String, CachedProviderResolution>> {
    SHOW_ID_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

#[derive(Debug, Clone)]
pub struct ResolverContext {
    pub identity: AnimeIdentity,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub translation_mode: AnimeTranslationMode,
    pub provider: AnimeProvider,
    pub simulate_provider_timeouts: u8,
}

#[derive(Debug, Clone)]
pub struct ProviderCatalogEntry {
    pub provider: AnimeProvider,
    pub provider_show_id: String,
    pub alias: String,
    pub episode_number: Option<i32>,
    pub sub_url: Option<String>,
    pub dub_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolverResult {
    pub source: AnimePlaybackSource,
    pub selected_translation: AnimeTranslationMode,
    pub fallback_used: bool,
    pub attempts: u8,
}

#[derive(Debug, Clone)]
pub struct ResolverConfig {
    pub max_retries: u8,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self { max_retries: 2 }
    }
}

#[derive(Debug, Clone)]
pub struct AnimeResolverService {
    config: ResolverConfig,
}

impl AnimeResolverService {
    pub fn new() -> Self {
        Self {
            config: ResolverConfig::default(),
        }
    }

    pub fn with_config(config: ResolverConfig) -> Self {
        Self { config }
    }

    pub fn generate_title_candidates(&self, identity: &AnimeIdentity) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut output = Vec::new();

        let mut push = |value: Option<&str>| {
            if let Some(value) = value {
                let normalized = value.trim();
                if normalized.is_empty() {
                    return;
                }
                let key = normalized.to_lowercase();
                if seen.insert(key) {
                    output.push(normalized.to_string());
                }
            }
        };

        push(Some(identity.canonical_title.as_str()));
        push(identity.romaji_title.as_deref());
        push(identity.english_title.as_deref());
        push(identity.native_title.as_deref());
        for alias in &identity.title_aliases {
            push(Some(alias));
        }

        for override_alias in overrides::title_overrides(identity.canonical_title.as_str()) {
            push(Some(override_alias));
        }

        output
    }

    pub fn resolve(
        &self,
        context: &ResolverContext,
        catalog: &[ProviderCatalogEntry],
    ) -> Result<ResolverResult, AnimeCommandError> {
        let candidates = self.generate_title_candidates(&context.identity);

        let mut timeout_remaining = context.simulate_provider_timeouts;
        let mut attempts = 0u8;

        while attempts <= self.config.max_retries {
            attempts += 1;

            if timeout_remaining > 0 {
                timeout_remaining -= 1;
                if attempts <= self.config.max_retries {
                    continue;
                }
                return Err(AnimeCommandError {
                    category: AnimeErrorCategory::ProviderSearchFailed,
                    message: "provider search timed out after retries".to_string(),
                    context: Some(format!("attempts={attempts}")),
                });
            }

            let found = self.provider_search(context.provider.clone(), &candidates, catalog)?;
            let best = self.select_best_match(found, &candidates, context.episode_number)?;

            let (url, selected_translation, fallback_used) =
                select_translation_source(&best, &context.translation_mode)?;

            return Ok(ResolverResult {
                source: AnimePlaybackSource {
                    provider: best.provider,
                    playback_kind: infer_playback_kind(&url),
                    url,
                    referer: Some("https://resolver.internal".to_string()),
                    subtitle_candidates: vec![DiscoveredSubtitle {
                        language: "en".to_string(),
                        label: Some("English".to_string()),
                        url: None,
                    }],
                    is_downloadable: true,
                },
                selected_translation,
                fallback_used,
                attempts,
            });
        }

        Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "provider search exhausted retries".to_string(),
            context: Some(format!("attempts={attempts}")),
        })
    }

    pub async fn resolve_live(
        &self,
        context: &ResolverContext,
    ) -> Result<ResolverResult, AnimeCommandError> {
        let candidates = self.generate_title_candidates(&context.identity);
        let client = http_client();
        let episode_number = context.episode_number.unwrap_or(1);

        let attempts: Vec<(&str, &str)> = match context.translation_mode {
            AnimeTranslationMode::Sub => vec![("sub", "sub"), ("dub", "sub")],
            AnimeTranslationMode::Dub => vec![
                ("dub", "dub"),
                ("sub", "dub"),
                ("dub", "sub"),
                ("sub", "sub"),
            ],
        };

        let mut resolved: Option<(String, Option<String>, Option<String>)> = None;
        for (search_type, stream_type) in attempts {
            let show_id = match fetch_allanime_show_id(client, &candidates, search_type).await? {
                Some(show_id) => show_id,
                None => continue,
            };

            let url = fetch_cached_or_live_stream_url(client, &show_id, stream_type, episode_number).await?;
            if let Some(url) = url {
                let (sub_url, dub_url) = match stream_type {
                    "sub" => (Some(url), None),
                    _ => (None, Some(url)),
                };
                resolved = Some((show_id, sub_url, dub_url));
                break;
            }
        }

        let (show_id, sub_url, dub_url) = resolved.ok_or_else(|| AnimeCommandError {
            category: AnimeErrorCategory::PlayableSourceMissing,
            message: "no playable source resolved after retry plan".to_string(),
            context: Some(format!("candidates={:?}; episode={episode_number}", candidates)),
        })?;

        let entry = ProviderCatalogEntry {
            provider: AnimeProvider::Gogoanime,
            provider_show_id: show_id,
            alias: candidates
                .first()
                .cloned()
                .unwrap_or_else(|| context.identity.canonical_title.clone()),
            episode_number: Some(episode_number),
            sub_url,
            dub_url,
        };

        let (url, selected_translation, fallback_used) =
            select_translation_source(&entry, &context.translation_mode)?;

        Ok(ResolverResult {
            source: AnimePlaybackSource {
                provider: entry.provider,
                playback_kind: infer_playback_kind(&url),
                url,
                referer: Some("https://allmanga.to".to_string()),
                subtitle_candidates: vec![DiscoveredSubtitle {
                    language: "en".to_string(),
                    label: Some("English".to_string()),
                    url: None,
                }],
                is_downloadable: true,
            },
            selected_translation,
            fallback_used,
            attempts: 1,
        })
    }

    fn provider_search(
        &self,
        provider: AnimeProvider,
        candidates: &[String],
        catalog: &[ProviderCatalogEntry],
    ) -> Result<Vec<ProviderCatalogEntry>, AnimeCommandError> {
        let matches: Vec<ProviderCatalogEntry> = catalog
            .iter()
            .filter(|entry| entry.provider == provider)
            .filter(|entry| {
                candidates
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(entry.alias.as_str()))
            })
            .cloned()
            .collect();

        if matches.is_empty() {
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::ProviderSearchFailed,
                message: "provider search returned no matches".to_string(),
                context: Some(format!("candidate_count={}", candidates.len())),
            });
        }

        Ok(matches)
    }

    fn select_best_match(
        &self,
        entries: Vec<ProviderCatalogEntry>,
        candidates: &[String],
        episode_number: Option<i32>,
    ) -> Result<ProviderCatalogEntry, AnimeCommandError> {
        let mut index_by_alias = HashMap::new();
        for (index, candidate) in candidates.iter().enumerate() {
            index_by_alias.insert(candidate.to_lowercase(), index);
        }

        let filtered: Vec<ProviderCatalogEntry> = if let Some(target_episode) = episode_number {
            entries
                .into_iter()
                .filter(|entry| entry.episode_number.unwrap_or(target_episode) == target_episode)
                .collect()
        } else {
            entries
        };

        if filtered.is_empty() {
            return Err(AnimeCommandError {
                category: AnimeErrorCategory::ProviderEpisodeMissing,
                message: "provider result exists but target episode is missing".to_string(),
                context: episode_number.map(|ep| format!("episode={ep}")),
            });
        }

        let mut sorted = filtered;
        sorted.sort_by_key(|entry| {
            index_by_alias
                .get(&entry.alias.to_lowercase())
                .copied()
                .unwrap_or(usize::MAX)
        });

        Ok(sorted[0].clone())
    }
}

async fn fetch_cached_or_live_stream_url(
    client: &reqwest::Client,
    show_id: &str,
    translation_type: &str,
    episode_number: i32,
) -> Result<Option<String>, AnimeCommandError> {
    let key = format!("{show_id}:{translation_type}:{episode_number}");
    if let Some(hit) = provider_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
    {
        if Instant::now() < hit.expires_at {
            if hit.stream_url == NEGATIVE_CACHE_SENTINEL {
                return Ok(None);
            }
            return Ok(Some(hit.stream_url));
        }
    }

    let live = fetch_allanime_episode_stream_url(client, show_id, translation_type, episode_number).await?;
    if let Ok(mut cache) = provider_cache().lock() {
        cache.insert(
            key,
            CachedProviderResolution {
                stream_url: live.clone().unwrap_or_else(|| NEGATIVE_CACHE_SENTINEL.to_string()),
                expires_at: Instant::now()
                    + if live.is_some() {
                        Duration::from_secs(60 * 60 * 6)
                    } else {
                        Duration::from_secs(90)
                    },
            },
        );
    }

    Ok(live)
}

const SEARCH_GQL: &str = "query($search:SearchInput $limit:Int $page:Int $translationType:VaildTranslationTypeEnumType $countryOrigin:VaildCountryOriginEnumType){shows(search:$search limit:$limit page:$page translationType:$translationType countryOrigin:$countryOrigin){edges{_id name}}}";
const EPISODE_GQL: &str = "query($showId:String! $translationType:VaildTranslationTypeEnumType! $episodeString:String!){episode(showId:$showId translationType:$translationType episodeString:$episodeString){episodeString sourceUrls}}";
const EPISODE_GQL_HASH: &str = "d405d0edd690624b66baba3068e0edc3ac90f1597d898a1ec8db4e5c43c00fec";

async fn fetch_allanime_show_id(
    client: &reqwest::Client,
    candidates: &[String],
    translation_type: &str,
) -> Result<Option<String>, AnimeCommandError> {
    let cache_key = format!("{}::{}", translation_type, normalized_candidates_key(candidates));
    if let Some(hit) = show_id_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&cache_key).cloned())
    {
        if Instant::now() < hit.expires_at {
            if hit.stream_url == NEGATIVE_CACHE_SENTINEL {
                return Ok(None);
            }
            return Ok(Some(hit.stream_url));
        }
    }

    for candidate in candidates {
        let payload = serde_json::json!({
            "query": SEARCH_GQL,
            "variables": {
                "search": { "allowAdult": false, "allowUnknown": false, "query": candidate },
                "limit": 8,
                "page": 1,
                "translationType": translation_type,
                "countryOrigin": "ALL",
            }
        });

        let response = client
            .post("https://api.allanime.day/api")
            .header("Content-Type", "application/json")
            .header("Referer", "https://allmanga.to")
            .header("Origin", "https://allmanga.to")
            .timeout(SHOW_LOOKUP_TIMEOUT)
            .json(&payload)
            .send()
            .await
            .map_err(|err| AnimeCommandError {
                category: AnimeErrorCategory::ProviderSearchFailed,
                message: "provider search request failed".to_string(),
                context: Some(err.to_string()),
            })?;

        if !response.status().is_success() {
            continue;
        }

        let json: Value = response.json().await.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "provider search payload parse failed".to_string(),
            context: Some(err.to_string()),
        })?;

        if let Some(show_id) = json
            .get("data")
            .and_then(|v| v.get("shows"))
            .and_then(|v| v.get("edges"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("_id"))
            .and_then(|v| v.as_str())
        {
            if let Ok(mut cache) = show_id_cache().lock() {
                cache.insert(
                    cache_key,
                    CachedProviderResolution {
                        stream_url: show_id.to_string(),
                        expires_at: Instant::now() + Duration::from_secs(60 * 60 * 24),
                    },
                );
            }
            return Ok(Some(show_id.to_string()));
        }
    }

    if let Ok(mut cache) = show_id_cache().lock() {
        cache.insert(
            cache_key,
            CachedProviderResolution {
                stream_url: NEGATIVE_CACHE_SENTINEL.to_string(),
                expires_at: Instant::now() + Duration::from_secs(90),
            },
        );
    }

    Ok(None)
}

async fn fetch_allanime_episode_stream_url(
    client: &reqwest::Client,
    show_id: &str,
    translation_type: &str,
    episode_number: i32,
) -> Result<Option<String>, AnimeCommandError> {
    let mut episode_candidates = vec![episode_number.to_string()];
    if !episode_candidates[0].contains('.') {
        episode_candidates.push(format!("{}.0", episode_number));
    }

    for episode_string in episode_candidates {
        let body = fetch_allanime_episode_body(client, show_id, translation_type, &episode_string).await?;
        let mut source_urls = parse_episode_source_urls(&body);
        source_urls.sort_by_key(|entry| {
            let name = entry
                .get("sourceName")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let rank = source_priority_rank(name);
            let provider_priority = entry
                .get("priority")
                .and_then(|v| v.as_f64())
                .unwrap_or(999.0);
            (rank, (provider_priority * 10.0) as i64)
        });
        source_urls.truncate(MAX_SOURCE_ATTEMPTS);

        let mut webview_fallback: Option<String> = None;

        for entry in source_urls {
            if let Some(source_url) = entry.get("sourceUrl").and_then(|v| v.as_str()) {
                let source_name = entry
                    .get("sourceName")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if source_url.starts_with("http") {
                    if is_playable_video_url(source_url) {
                        return Ok(Some(source_url.to_string()));
                    }

                    if webview_fallback.is_none() && is_webview_source(source_name, source_url) {
                        webview_fallback = Some(source_url.to_string());
                    }

                    if source_name.eq_ignore_ascii_case("Yt-mp4")
                        || source_name.eq_ignore_ascii_case("Luf-Mp4")
                        || source_url.contains("fast4speed.rsvp")
                    {
                        if let Some(url) = follow_redirect_final_url(client, source_url).await? {
                            if is_playable_video_url(&url) {
                                return Ok(Some(url));
                            }
                        }
                    }
                    continue;
                }

                if !source_url.starts_with("--") {
                    continue;
                }

                let decoded = decode_allanime_url(source_url);
                let mut clock_url = decoded.replace("/clock", "/clock.json");

                if clock_url.contains("fast4speed.rsvp") || source_name.eq_ignore_ascii_case("Yt-mp4") {
                    if let Some(url) = follow_redirect_final_url(client, &clock_url).await? {
                        if is_playable_video_url(&url) {
                            return Ok(Some(url));
                        }
                    }
                    continue;
                }

                if clock_url.starts_with("//") {
                    clock_url = format!("https:{clock_url}");
                } else if clock_url.starts_with('/') {
                    clock_url = format!("https://allanime.day{clock_url}");
                } else if !clock_url.starts_with("http") {
                    clock_url = format!("https://allanime.day/{clock_url}");
                }

                if let Some(stream_url) = extract_stream_url_from_clock(client, &clock_url).await? {
                    return Ok(Some(stream_url));
                }
            }
        }

        if let Some(url) = webview_fallback {
            return Ok(Some(url));
        }
    }

    Ok(None)
}

async fn fetch_allanime_episode_body(
    client: &reqwest::Client,
    show_id: &str,
    translation_type: &str,
    episode_string: &str,
) -> Result<String, AnimeCommandError> {
    let variables = serde_json::json!({
        "showId": show_id,
        "translationType": translation_type,
        "episodeString": episode_string,
    });

    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": EPISODE_GQL_HASH,
        }
    });

    let get_response = client
        .get("https://api.allanime.day/api")
        .header("Referer", "https://allmanga.to")
        .header("Origin", "https://youtu-chan.com")
        .timeout(EPISODE_FETCH_TIMEOUT)
        .query(&[
            ("variables", variables.to_string()),
            ("extensions", extensions.to_string()),
        ])
        .send()
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "provider episode persisted query request failed".to_string(),
            context: Some(err.to_string()),
        })?;

    if get_response.status().is_success() {
        let get_body = get_response.text().await.map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "provider episode persisted query payload parse failed".to_string(),
            context: Some(err.to_string()),
        })?;

        if !parse_episode_source_urls(&get_body).is_empty() || get_body.contains("tobeparsed") {
            return Ok(get_body);
        }
    }

    let payload = serde_json::json!({
        "query": EPISODE_GQL,
        "variables": variables,
    });

    let post_response = client
        .post("https://api.allanime.day/api")
        .header("Content-Type", "application/json")
        .header("Referer", "https://allmanga.to")
        .header("Origin", "https://allmanga.to")
        .timeout(EPISODE_FETCH_TIMEOUT)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "provider episode request failed".to_string(),
            context: Some(err.to_string()),
        })?;

    if !post_response.status().is_success() {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::ProviderSearchFailed,
            message: "provider episode returned non-success status".to_string(),
            context: Some(post_response.status().to_string()),
        });
    }

    post_response.text().await.map_err(|err| AnimeCommandError {
        category: AnimeErrorCategory::ProviderSearchFailed,
        message: "provider episode payload parse failed".to_string(),
        context: Some(err.to_string()),
    })
}

fn parse_episode_source_urls(body: &str) -> Vec<Value> {
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        let direct = json
            .get("data")
            .and_then(|v| v.get("episode"))
            .and_then(|v| v.get("sourceUrls"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if !direct.is_empty() {
            return direct;
        }
    }

    let re = Regex::new(r#""tobeparsed"\s*:\s*"([^"]+)""#).ok();
    let Some(re) = re else { return vec![] };
    let Some(captures) = re.captures(body) else { return vec![] };
    let Some(blob) = captures.get(1).map(|m| m.as_str()) else { return vec![] };
    decode_tobeparsed(blob)
}

fn decode_tobeparsed(blob: &str) -> Vec<Value> {
    type Aes256Ctr = ctr::Ctr128BE<Aes256>;
    let raw = match base64::engine::general_purpose::STANDARD.decode(blob) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    if raw.len() <= 29 {
        return vec![];
    }

    let key = Sha256::digest(b"Xot36i3lK3:v1");
    let iv12 = &raw[1..13];
    let mut iv16 = [0u8; 16];
    iv16[..12].copy_from_slice(iv12);
    iv16[15] = 2;
    let mut ciphertext = raw[13..raw.len().saturating_sub(16)].to_vec();

    let mut cipher = Aes256Ctr::new((&key).into(), (&iv16).into());
    cipher.apply_keystream(&mut ciphertext);
    let plain = match String::from_utf8(ciphertext) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    if let Ok(json) = serde_json::from_str::<Value>(&plain) {
        return json
            .get("episode")
            .and_then(|v| v.get("sourceUrls"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
    }

    let source_re = Regex::new(r#""sourceUrl"\s*:\s*"([^"]+)"(?:[^{}]*?"sourceName"\s*:\s*"([^"]*)")?"#).ok();
    let Some(source_re) = source_re else { return vec![] };

    source_re
        .captures_iter(&plain)
        .filter_map(|cap| {
            let source_url = cap.get(1)?.as_str();
            let source_name = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            Some(serde_json::json!({ "sourceUrl": source_url, "sourceName": source_name }))
        })
        .collect()
}

fn source_priority_rank(source_name: &str) -> i32 {
    match source_name {
        "S-mp4" => 0,
        "Luf-Mp4" => 1,
        "Yt-mp4" => 2,
        "Default" => 3,
        "Sl-Hls" => 4,
        "Vn-Hls" => 5,
        "Fm-Hls" => 6,
        "Mp4" => 7,
        "Ok" => 8,
        "Uni" => 9,
        _ => 10,
    }
}

fn is_webview_source(source_name: &str, source_url: &str) -> bool {
    source_name.eq_ignore_ascii_case("Mp4")
        || source_name.eq_ignore_ascii_case("Vn-Hls")
        || source_name.eq_ignore_ascii_case("Fm-Hls")
        || source_name.eq_ignore_ascii_case("Ok")
        || source_name.eq_ignore_ascii_case("Uni")
        || source_url.contains("mp4upload.com")
        || source_url.contains("vidnest.io")
        || source_url.contains("bysekoze.com")
        || source_url.contains("ok.ru")
        || source_url.contains("allanime.uns.bio")
}

async fn follow_redirect_final_url(
    client: &reqwest::Client,
    url: &str,
) -> Result<Option<String>, AnimeCommandError> {
    let response = match client
        .get(url)
        .header("Referer", "https://allmanga.to")
        .timeout(SOURCE_ATTEMPT_TIMEOUT)
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };
    Ok(Some(response.url().to_string()))
}

fn is_playable_video_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains(".mp4")
        || lower.contains(".m3u8")
        || lower.contains("googlevideo.com")
        || lower.contains(".webm")
        || lower.contains(".mkv")
}

fn infer_playback_kind(url: &str) -> AnimePlaybackKind {
    let lower = url.to_lowercase();
    if lower.contains(".m3u8") {
        AnimePlaybackKind::Hls
    } else if lower.contains(".mp4") || lower.contains("googlevideo.com") || lower.contains(".webm") {
        AnimePlaybackKind::DirectVideo
    } else {
        AnimePlaybackKind::WebviewRemote
    }
}

async fn extract_stream_url_from_clock(
    client: &reqwest::Client,
    clock_url: &str,
) -> Result<Option<String>, AnimeCommandError> {
    let response = match client
        .get(clock_url)
        .header("Referer", "https://allmanga.to")
        .timeout(SOURCE_ATTEMPT_TIMEOUT)
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };

    if !response.status().is_success() {
        return Ok(None);
    }

    let json: Value = match response.json().await {
        Ok(json) => json,
        Err(_) => return Ok(None),
    };

    if let Some(link) = json
        .get("links")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("link"))
        .and_then(|v| v.as_str())
    {
        return Ok(Some(link.to_string()));
    }

    Ok(None)
}

fn normalized_candidates_key(candidates: &[String]) -> String {
    let mut normalized: Vec<String> = candidates
        .iter()
        .map(|candidate| candidate.trim().to_lowercase())
        .filter(|candidate| !candidate.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized.join("|")
}

fn decode_allanime_url(encoded: &str) -> String {
    let input = encoded.trim_start_matches("--");
    let map: HashMap<&'static str, &'static str> = [
        ("79", "A"), ("7a", "B"), ("7b", "C"), ("7c", "D"), ("7d", "E"), ("7e", "F"), ("7f", "G"),
        ("70", "H"), ("71", "I"), ("72", "J"), ("73", "K"), ("74", "L"), ("75", "M"), ("76", "N"),
        ("77", "O"), ("68", "P"), ("69", "Q"), ("6a", "R"), ("6b", "S"), ("6c", "T"), ("6d", "U"),
        ("6e", "V"), ("6f", "W"), ("60", "X"), ("61", "Y"), ("62", "Z"), ("59", "a"), ("5a", "b"),
        ("5b", "c"), ("5c", "d"), ("5d", "e"), ("5e", "f"), ("5f", "g"), ("50", "h"), ("51", "i"),
        ("52", "j"), ("53", "k"), ("54", "l"), ("55", "m"), ("56", "n"), ("57", "o"), ("48", "p"),
        ("49", "q"), ("4a", "r"), ("4b", "s"), ("4c", "t"), ("4d", "u"), ("4e", "v"), ("4f", "w"),
        ("40", "x"), ("41", "y"), ("42", "z"), ("08", "0"), ("09", "1"), ("0a", "2"), ("0b", "3"),
        ("0c", "4"), ("0d", "5"), ("0e", "6"), ("0f", "7"), ("00", "8"), ("01", "9"), ("15", "-"),
        ("16", "."), ("67", "_"), ("46", "~"), ("02", ":"), ("17", "/"),
        ("07", "?"), ("1b", "#"), ("63", "["), ("65", "]"), ("78", "@"),
        ("19", "!"), ("1c", "$"), ("1e", "&"), ("10", "("), ("11", ")"),
        ("12", "*"), ("13", "+"), ("14", ","), ("03", ";"), ("05", "="),
        ("1d", "%")
    ].into_iter().collect();

    let mut output = String::new();
    for i in (0..input.len()).step_by(2) {
        if i + 2 > input.len() { break; }
        let pair = &input[i..i + 2];
        output.push_str(map.get(pair).copied().unwrap_or(pair));
    }
    output.replace("\\u002F", "/").replace("\\|", "")
}

fn select_translation_source(
    entry: &ProviderCatalogEntry,
    preferred: &AnimeTranslationMode,
) -> Result<(String, AnimeTranslationMode, bool), AnimeCommandError> {
    match preferred {
        AnimeTranslationMode::Sub => {
            if let Some(url) = &entry.sub_url {
                return Ok((url.clone(), AnimeTranslationMode::Sub, false));
            }
            Err(AnimeCommandError {
                category: AnimeErrorCategory::PlayableSourceMissing,
                message: "sub source missing for selected episode".to_string(),
                context: Some(format!("provider_show_id={}", entry.provider_show_id)),
            })
        }
        AnimeTranslationMode::Dub => {
            if let Some(url) = &entry.dub_url {
                return Ok((url.clone(), AnimeTranslationMode::Dub, false));
            }
            if let Some(url) = &entry.sub_url {
                return Ok((url.clone(), AnimeTranslationMode::Sub, true));
            }
            Err(AnimeCommandError {
                category: AnimeErrorCategory::TranslationUnavailable,
                message: "dub unavailable and sub fallback missing".to_string(),
                context: Some(format!("provider_show_id={}", entry.provider_show_id)),
            })
        }
    }
}

mod overrides {
    pub fn title_overrides(canonical_title: &str) -> &'static [&'static str] {
        match canonical_title.to_lowercase().as_str() {
            "shingeki no kyojin" => &["Attack on Titan"],
            "boku no hero academia" => &["My Hero Academia"],
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::anime::models::AnimeMediaKind;

    fn identity() -> AnimeIdentity {
        AnimeIdentity {
            media_kind: AnimeMediaKind::AnimeSeries,
            tmdb_id: Some(1),
            anilist_id: Some(2),
            mal_id: Some(3),
            canonical_title: "Shingeki no Kyojin".to_string(),
            romaji_title: Some("Shingeki no Kyojin".to_string()),
            english_title: Some("Attack on Titan".to_string()),
            native_title: None,
            title_aliases: vec!["AoT".to_string()],
        }
    }

    fn catalog(sub: bool, dub: bool) -> Vec<ProviderCatalogEntry> {
        vec![ProviderCatalogEntry {
            provider: AnimeProvider::Gogoanime,
            provider_show_id: "show-1".to_string(),
            alias: "Attack on Titan".to_string(),
            episode_number: Some(1),
            sub_url: sub.then(|| "https://stream/sub.m3u8".to_string()),
            dub_url: dub.then(|| "https://stream/dub.m3u8".to_string()),
        }]
    }

    #[test]
    fn resolves_sub_successfully() {
        let resolver = AnimeResolverService::new();
        let result = resolver
            .resolve(
                &ResolverContext {
                    identity: identity(),
                    season_number: Some(1),
                    episode_number: Some(1),
                    translation_mode: AnimeTranslationMode::Sub,
                    provider: AnimeProvider::Gogoanime,
                    simulate_provider_timeouts: 0,
                },
                &catalog(true, false),
            )
            .expect("sub should resolve");
        assert_eq!(result.source.url, "https://stream/sub.m3u8");
        assert!(!result.fallback_used);
    }

    #[test]
    fn resolves_dub_successfully() {
        let resolver = AnimeResolverService::new();
        let result = resolver
            .resolve(
                &ResolverContext {
                    identity: identity(),
                    season_number: Some(1),
                    episode_number: Some(1),
                    translation_mode: AnimeTranslationMode::Dub,
                    provider: AnimeProvider::Gogoanime,
                    simulate_provider_timeouts: 0,
                },
                &catalog(true, true),
            )
            .expect("dub should resolve");
        assert_eq!(result.selected_translation, AnimeTranslationMode::Dub);
    }

    #[test]
    fn falls_back_from_dub_to_sub() {
        let resolver = AnimeResolverService::new();
        let result = resolver
            .resolve(
                &ResolverContext {
                    identity: identity(),
                    season_number: Some(1),
                    episode_number: Some(1),
                    translation_mode: AnimeTranslationMode::Dub,
                    provider: AnimeProvider::Gogoanime,
                    simulate_provider_timeouts: 0,
                },
                &catalog(true, false),
            )
            .expect("dub should fallback to sub");
        assert_eq!(result.selected_translation, AnimeTranslationMode::Sub);
        assert!(result.fallback_used);
    }

    #[test]
    fn returns_typed_error_when_no_source_exists() {
        let resolver = AnimeResolverService::new();
        let error = resolver
            .resolve(
                &ResolverContext {
                    identity: identity(),
                    season_number: Some(1),
                    episode_number: Some(1),
                    translation_mode: AnimeTranslationMode::Dub,
                    provider: AnimeProvider::Gogoanime,
                    simulate_provider_timeouts: 0,
                },
                &catalog(false, false),
            )
            .expect_err("missing sources should error");
        assert!(matches!(error.category, AnimeErrorCategory::TranslationUnavailable));
    }

    #[test]
    fn retries_on_provider_timeout() {
        let resolver = AnimeResolverService::with_config(ResolverConfig { max_retries: 2 });
        let result = resolver
            .resolve(
                &ResolverContext {
                    identity: identity(),
                    season_number: Some(1),
                    episode_number: Some(1),
                    translation_mode: AnimeTranslationMode::Sub,
                    provider: AnimeProvider::Gogoanime,
                    simulate_provider_timeouts: 1,
                },
                &catalog(true, false),
            )
            .expect("should succeed after one retry");
        assert_eq!(result.attempts, 2);
    }

    #[test]
    fn prefers_first_title_candidate_on_alias_collision() {
        let resolver = AnimeResolverService::new();
        let mut collision_catalog = catalog(true, false);
        collision_catalog.push(ProviderCatalogEntry {
            provider: AnimeProvider::Gogoanime,
            provider_show_id: "show-2".to_string(),
            alias: "AoT".to_string(),
            episode_number: Some(1),
            sub_url: Some("https://stream/collision.m3u8".to_string()),
            dub_url: None,
        });

        let result = resolver
            .resolve(
                &ResolverContext {
                    identity: identity(),
                    season_number: Some(1),
                    episode_number: Some(1),
                    translation_mode: AnimeTranslationMode::Sub,
                    provider: AnimeProvider::Gogoanime,
                    simulate_provider_timeouts: 0,
                },
                &collision_catalog,
            )
            .expect("collision should still resolve");

        assert_eq!(result.source.url, "https://stream/sub.m3u8");
    }
}
