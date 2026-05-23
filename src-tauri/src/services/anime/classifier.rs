use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeClassificationInput {
    pub has_animation_genre: bool,
    pub original_language: Option<String>,
    pub origin_countries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeClassificationResult {
    pub is_anime: bool,
    pub confidence: f32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AnimeClassifierService;

impl AnimeClassifierService {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, input: &AnimeClassificationInput) -> AnimeClassificationResult {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        if input.has_animation_genre {
            score += 0.6;
            reasons.push("animation genre signal".to_string());
        }

        if input
            .original_language
            .as_deref()
            .map(|lang| lang.eq_ignore_ascii_case("ja"))
            .unwrap_or(false)
        {
            score += 0.3;
            reasons.push("japanese language signal".to_string());
        }

        if input
            .origin_countries
            .iter()
            .any(|country| country.eq_ignore_ascii_case("JP"))
        {
            score += 0.2;
            reasons.push("japanese origin signal".to_string());
        }

        if score > 1.0 {
            score = 1.0;
        }

        AnimeClassificationResult {
            is_anime: score >= 0.6,
            confidence: score,
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_positive_when_animation_and_japanese_signal_present() {
        let service = AnimeClassifierService::new();
        let input = AnimeClassificationInput {
            has_animation_genre: true,
            original_language: Some("ja".to_string()),
            origin_countries: vec!["JP".to_string()],
        };

        let result = service.classify(&input);

        assert!(result.is_anime);
        assert!(result.confidence >= 0.9);
        assert!(!result.reasons.is_empty());
    }

    #[test]
    fn classifies_negative_when_no_signals_present() {
        let service = AnimeClassifierService::new();
        let input = AnimeClassificationInput {
            has_animation_genre: false,
            original_language: Some("en".to_string()),
            origin_countries: vec!["US".to_string()],
        };

        let result = service.classify(&input);

        assert!(!result.is_anime);
        assert_eq!(result.confidence, 0.0);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn classifies_edge_case_animation_only_as_anime() {
        let service = AnimeClassifierService::new();
        let input = AnimeClassificationInput {
            has_animation_genre: true,
            original_language: None,
            origin_countries: vec![],
        };

        let result = service.classify(&input);

        assert!(result.is_anime);
        assert_eq!(result.confidence, 0.6);
        assert_eq!(result.reasons.len(), 1);
    }
}
