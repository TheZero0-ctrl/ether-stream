use serde::Deserialize;

use super::models::{AnimeSkipTimings, SkipSegment, SkipSegmentKind};

#[derive(Debug, Clone, Default)]
pub struct AnimeSkipService;

#[derive(Debug, Clone)]
pub enum AnimeSkipMode {
    Off,
    Auto,
    Manual,
}

#[derive(Deserialize)]
struct AniSkipResponse {
    results: Option<Vec<AniSkipSegment>>,
}

#[derive(Deserialize)]
struct AniSkipSegment {
    #[serde(rename = "skip_type")]
    skip_type: String,
    interval: AniSkipInterval,
}

#[derive(Deserialize)]
struct AniSkipInterval {
    start_time: f64,
    end_time: f64,
}

impl AnimeSkipService {
    pub fn new() -> Self {
        Self
    }

    pub async fn fetch_timings(
        &self,
        mal_id: i64,
        episode_number: i32,
    ) -> Result<AnimeSkipTimings, String> {
        let url = format!(
            "https://api.aniskip.com/v2/skip-times/{}/{}/?types[]=op&types[]=ed",
            mal_id, episode_number
        );

        let response = reqwest::Client::new()
            .get(url)
            .send()
            .await
            .map_err(|err| format!("aniskip request failed: {err}"))?;

        if !response.status().is_success() {
            return Err(format!("aniskip returned status {}", response.status()));
        }

        let payload: AniSkipResponse = response
            .json()
            .await
            .map_err(|err| format!("aniskip payload parse failed: {err}"))?;

        let segments = normalize_segments(payload.results.unwrap_or_default());

        Ok(AnimeSkipTimings {
            mal_id,
            episode_number,
            segments,
        })
    }

    pub fn can_emit_active_segment(
        &self,
        mode: AnimeSkipMode,
        has_identity: bool,
        is_seekable: bool,
    ) -> bool {
        if !has_identity || !is_seekable {
            return false;
        }
        !matches!(mode, AnimeSkipMode::Off)
    }
}

fn normalize_segments(raw: Vec<AniSkipSegment>) -> Vec<SkipSegment> {
    raw.into_iter()
        .filter_map(|segment| {
            let kind = match segment.skip_type.as_str() {
                "op" | "mixed-op" => Some(SkipSegmentKind::Intro),
                "ed" | "mixed-ed" => Some(SkipSegmentKind::Outro),
                _ => None,
            }?;

            if segment.interval.end_time <= segment.interval.start_time {
                return None;
            }

            Some(SkipSegment {
                kind,
                start_seconds: segment.interval.start_time,
                end_seconds: segment.interval.end_time,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gates_skip_activation_by_mode_and_seekability() {
        let service = AnimeSkipService::new();
        assert!(!service.can_emit_active_segment(AnimeSkipMode::Off, true, true));
        assert!(service.can_emit_active_segment(AnimeSkipMode::Auto, true, true));
        assert!(service.can_emit_active_segment(AnimeSkipMode::Manual, true, true));
        assert!(!service.can_emit_active_segment(AnimeSkipMode::Auto, false, true));
        assert!(!service.can_emit_active_segment(AnimeSkipMode::Auto, true, false));
    }

    #[test]
    fn filters_invalid_boundary_segments() {
        let normalized = normalize_segments(vec![AniSkipSegment {
            skip_type: "op".to_string(),
            interval: AniSkipInterval {
                start_time: 90.0,
                end_time: 90.0,
            },
        }]);

        assert!(normalized.is_empty());
    }

    #[test]
    fn handles_no_data_behavior() {
        let normalized = normalize_segments(vec![]);
        assert!(normalized.is_empty());
    }
}
