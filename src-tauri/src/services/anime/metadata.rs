use std::collections::BTreeSet;

use super::models::{
    AnimeDetails, AnimeIdentity, AnimeMediaKind, AnimeSourceConfidence,
};

#[derive(Debug, Clone)]
pub struct TmdbMetadataInput {
    pub tmdb_id: Option<i64>,
    pub media_kind: AnimeMediaKind,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
    pub release_year: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnilistCandidate {
    pub anilist_id: i64,
    pub mal_id: Option<i64>,
    pub canonical_title: String,
    pub romaji_title: Option<String>,
    pub english_title: Option<String>,
    pub native_title: Option<String>,
    pub aliases: Vec<String>,
    pub score: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum AnilistLookupResult {
    Found(AnilistCandidate),
    Ambiguous(Vec<AnilistCandidate>),
    Missing,
    Cached(AnilistCandidate),
}

#[derive(Debug, Clone)]
pub struct MetadataEnrichmentOutput {
    pub details: AnimeDetails,
    pub match_state: MetadataMatchState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataMatchState {
    Found,
    Cached,
}

#[derive(Debug, Clone, Default)]
pub struct AnimeMetadataService;

impl AnimeMetadataService {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize_tmdb_input(&self, input: TmdbMetadataInput) -> TmdbMetadataInput {
        TmdbMetadataInput {
            tmdb_id: input.tmdb_id,
            media_kind: input.media_kind,
            title: input.title.trim().to_string(),
            overview: input.overview.map(|value| value.trim().to_string()),
            poster_url: input.poster_url.map(|value| value.trim().to_string()),
            backdrop_url: input.backdrop_url.map(|value| value.trim().to_string()),
            genres: input
                .genres
                .into_iter()
                .map(|genre| genre.trim().to_string())
                .filter(|genre| !genre.is_empty())
                .collect(),
            release_year: input.release_year,
            status: input.status.map(|value| value.trim().to_string()),
        }
    }

    pub fn enrich(
        &self,
        tmdb: TmdbMetadataInput,
        anilist_lookup: AnilistLookupResult,
    ) -> Result<MetadataEnrichmentOutput, MetadataEnrichmentError> {
        let normalized_tmdb = self.normalize_tmdb_input(tmdb);

        match anilist_lookup {
            AnilistLookupResult::Missing => Err(MetadataEnrichmentError::MissingMatch),
            AnilistLookupResult::Ambiguous(candidates) => {
                Err(MetadataEnrichmentError::AmbiguousMatch {
                    candidate_ids: candidates.into_iter().map(|value| value.anilist_id).collect(),
                })
            }
            AnilistLookupResult::Found(candidate) => Ok(MetadataEnrichmentOutput {
                details: build_anime_details(normalized_tmdb, candidate),
                match_state: MetadataMatchState::Found,
            }),
            AnilistLookupResult::Cached(candidate) => Ok(MetadataEnrichmentOutput {
                details: build_anime_details(normalized_tmdb, candidate),
                match_state: MetadataMatchState::Cached,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MetadataEnrichmentError {
    MissingMatch,
    AmbiguousMatch { candidate_ids: Vec<i64> },
}

fn build_anime_details(tmdb: TmdbMetadataInput, candidate: AnilistCandidate) -> AnimeDetails {
    let aliases = normalized_aliases(&tmdb.title, &candidate);

    AnimeDetails {
        identity: AnimeIdentity {
            media_kind: tmdb.media_kind,
            tmdb_id: tmdb.tmdb_id,
            anilist_id: Some(candidate.anilist_id),
            mal_id: candidate.mal_id,
            canonical_title: candidate.canonical_title,
            romaji_title: candidate.romaji_title,
            english_title: candidate.english_title,
            native_title: candidate.native_title,
            title_aliases: aliases,
        },
        overview: tmdb.overview,
        poster_url: tmdb.poster_url,
        backdrop_url: tmdb.backdrop_url,
        genres: tmdb.genres,
        score: candidate.score,
        release_year: tmdb.release_year,
        status: tmdb.status,
        source_confidence: AnimeSourceConfidence::High,
    }
}

fn normalized_aliases(tmdb_title: &str, candidate: &AnilistCandidate) -> Vec<String> {
    let mut dedup = BTreeSet::new();
    let mut output = Vec::new();

    for value in std::iter::once(tmdb_title)
        .chain(std::iter::once(candidate.canonical_title.as_str()))
        .chain(candidate.romaji_title.as_deref())
        .chain(candidate.english_title.as_deref())
        .chain(candidate.native_title.as_deref())
        .chain(candidate.aliases.iter().map(String::as_str))
    {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }

        let key = normalized.to_lowercase();
        if dedup.insert(key) {
            output.push(normalized.to_string());
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmdb_input() -> TmdbMetadataInput {
        TmdbMetadataInput {
            tmdb_id: Some(11),
            media_kind: AnimeMediaKind::AnimeSeries,
            title: "  Attack on Titan ".to_string(),
            overview: Some("  Titans attack humanity.  ".to_string()),
            poster_url: Some(" https://img/poster.jpg ".to_string()),
            backdrop_url: Some(" https://img/backdrop.jpg ".to_string()),
            genres: vec![" Animation ".to_string(), " Action ".to_string()],
            release_year: Some(2013),
            status: Some(" Finished ".to_string()),
        }
    }

    fn found_candidate() -> AnilistCandidate {
        AnilistCandidate {
            anilist_id: 16498,
            mal_id: Some(16498),
            canonical_title: "Shingeki no Kyojin".to_string(),
            romaji_title: Some("Shingeki no Kyojin".to_string()),
            english_title: Some("Attack on Titan".to_string()),
            native_title: Some("進撃の巨人".to_string()),
            aliases: vec!["AoT".to_string(), "Attack on Titan".to_string()],
            score: Some(86.5),
        }
    }

    #[test]
    fn enriches_found_match_into_details() {
        let service = AnimeMetadataService::new();
        let output = service
            .enrich(tmdb_input(), AnilistLookupResult::Found(found_candidate()))
            .expect("expected found match output");

        assert_eq!(output.match_state, MetadataMatchState::Found);
        assert_eq!(output.details.identity.anilist_id, Some(16498));
        assert!(output
            .details
            .identity
            .title_aliases
            .contains(&"Attack on Titan".to_string()));
        assert_eq!(output.details.overview.as_deref(), Some("Titans attack humanity."));
    }

    #[test]
    fn returns_ambiguous_error_for_multiple_candidates() {
        let service = AnimeMetadataService::new();
        let result = service.enrich(
            tmdb_input(),
            AnilistLookupResult::Ambiguous(vec![
                found_candidate(),
                AnilistCandidate {
                    anilist_id: 20000,
                    ..found_candidate()
                },
            ]),
        );

        match result {
            Err(MetadataEnrichmentError::AmbiguousMatch { candidate_ids }) => {
                assert_eq!(candidate_ids.len(), 2);
            }
            _ => panic!("expected ambiguous match error"),
        }
    }

    #[test]
    fn returns_missing_error_when_no_match_exists() {
        let service = AnimeMetadataService::new();
        let result = service.enrich(tmdb_input(), AnilistLookupResult::Missing);

        match result {
            Err(MetadataEnrichmentError::MissingMatch) => {}
            _ => panic!("expected missing match error"),
        }
    }

    #[test]
    fn enriches_cached_match_and_marks_state() {
        let service = AnimeMetadataService::new();
        let output = service
            .enrich(tmdb_input(), AnilistLookupResult::Cached(found_candidate()))
            .expect("expected cached match output");

        assert_eq!(output.match_state, MetadataMatchState::Cached);
        assert_eq!(output.details.identity.mal_id, Some(16498));
    }
}
