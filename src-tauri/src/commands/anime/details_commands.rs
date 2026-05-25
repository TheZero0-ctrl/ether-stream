use tauri::State;

use crate::database::AppDatabase;
use crate::services::anime::classifier::{AnimeClassificationInput, AnimeClassifierService};
use crate::services::anime::errors::{AnimeCommandError, AnimeErrorCategory};
use crate::services::anime::metadata::{
    lookup_anilist_candidate, AnilistCandidate, AnilistLookupResult, AnimeMetadataService,
    MetadataEnrichmentError, TmdbMetadataInput,
};
use crate::services::anime::models::{
    AnimeDetails, AnimeMediaKind, AnimeSourceConfidence,
};
use crate::services::anime::repository_sqlx::AnimeSqlxRepository;

use super::{AnimeGetDetailsRequest, AnimeGetDetailsResponse};
use crate::services::anime::playback::build_identity_key;

pub(super) async fn handle_anime_get_details(
    request: AnimeGetDetailsRequest,
    db: State<'_, AppDatabase>,
) -> Result<AnimeGetDetailsResponse, AnimeCommandError> {
    let classifier = AnimeClassifierService::new();
    let classification = classifier.classify(&AnimeClassificationInput {
        has_animation_genre: request.has_animation_genre,
        original_language: request.original_language.clone(),
        origin_countries: request.origin_countries.clone(),
    });

    if !classification.is_anime {
        return Err(AnimeCommandError {
            category: AnimeErrorCategory::AnimeNotClassified,
            message: "media item does not meet anime classification threshold".to_string(),
            context: Some(format!(
                "confidence={:.2}; reasons={}"
                , classification.confidence,
                classification.reasons.join(", ")
            )),
        });
    }

    let metadata_service = AnimeMetadataService::new();

    let tmdb_input = TmdbMetadataInput {
        tmdb_id: request.tmdb_id,
        media_kind: AnimeMediaKind::AnimeSeries,
        title: request
            .title
            .unwrap_or_else(|| "Unknown Anime".to_string()),
        overview: request.overview,
        poster_url: request.poster_url,
        backdrop_url: request.backdrop_url,
        genres: request.genres.unwrap_or_default(),
        release_year: request.release_year,
        status: request.status,
    };

    let anilist_lookup = if let Some(anilist_id) = request.anilist_id {
        AnilistLookupResult::Found(AnilistCandidate {
            anilist_id,
            mal_id: request.mal_id,
            canonical_title: tmdb_input.title.clone(),
            romaji_title: None,
            english_title: Some(tmdb_input.title.clone()),
            native_title: None,
            aliases: vec![tmdb_input.title.clone()],
            score: None,
            total_episodes: None,
        })
    } else {
        match lookup_anilist_candidate(&tmdb_input.title).await {
            Ok(Some(candidate)) => AnilistLookupResult::Found(candidate),
            Ok(None) => AnilistLookupResult::Missing,
            Err(error) => {
                return Err(AnimeCommandError {
                    category: AnimeErrorCategory::AnilistMatchMissing,
                    message: "anilist lookup failed".to_string(),
                    context: Some(error),
                });
            }
        }
    };

    match metadata_service.enrich(tmdb_input, anilist_lookup) {
        Ok(output) => Ok(AnimeGetDetailsResponse {
            details: {
                let details = AnimeDetails {
                source_confidence: if classification.confidence >= 0.85 {
                    AnimeSourceConfidence::High
                } else if classification.confidence >= 0.7 {
                    AnimeSourceConfidence::Medium
                } else {
                    AnimeSourceConfidence::Low
                },
                ..output.details
                };

                persist_resolved_identity(&db, &details.identity).await?;
                details
            },
        }),
        Err(MetadataEnrichmentError::MissingMatch) => Err(AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "no AniList match found for anime metadata enrichment".to_string(),
            context: request.tmdb_id.map(|id| format!("tmdb_id={id}")),
        }),
        Err(MetadataEnrichmentError::AmbiguousMatch { candidate_ids }) => Err(AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "multiple AniList candidates found; reconciliation needed".to_string(),
            context: Some(format!("candidate_ids={candidate_ids:?}")),
        }),
    }
}

async fn persist_resolved_identity(
    db: &State<'_, AppDatabase>,
    identity: &crate::services::anime::models::AnimeIdentity,
) -> Result<(), AnimeCommandError> {
    if identity.anilist_id.is_none() && identity.mal_id.is_none() {
        return Ok(());
    }

    let identity_key = build_identity_key(identity);
    let repository = AnimeSqlxRepository::new(db.0.clone());

    repository
        .upsert_identity(&identity_key, identity)
        .await
        .map_err(|err| AnimeCommandError {
            category: AnimeErrorCategory::AnilistMatchMissing,
            message: "failed to persist resolved anime identity".to_string(),
            context: Some(err.to_string()),
        })
}
