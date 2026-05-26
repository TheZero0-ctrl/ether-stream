use serde::{Deserialize, Serialize};

use super::models::{
    AnimeEpisode, AnimeProvider, AnimeSeason, AnimeSeasonSourceKind, ProviderEpisodeRef,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingEpisodeInput {
    pub tmdb_season_number: Option<i32>,
    pub tmdb_episode_number: Option<i32>,
    pub anilist_episode_number: Option<i32>,
    pub title: Option<String>,
    pub runtime_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingInput {
    pub is_movie: bool,
    pub tmdb_episodes: Vec<MappingEpisodeInput>,
    pub anilist_episode_count: Option<i32>,
    pub released_episode_count: Option<i32>,
    pub provider: Option<AnimeProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingDiagnostics {
    pub source_kind: AnimeSeasonSourceKind,
    pub fallback_used: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingOutput {
    pub seasons: Vec<AnimeSeason>,
    pub diagnostics: MappingDiagnostics,
}

#[derive(Debug, Clone, Default)]
pub struct AnimeMappingService;

impl AnimeMappingService {
    pub fn new() -> Self {
        Self
    }

    pub fn map(&self, input: MappingInput) -> MappingOutput {
        if input.is_movie {
            return self.map_movie(input);
        }
        self.map_series(input)
    }

    fn map_movie(&self, input: MappingInput) -> MappingOutput {
        let episode = AnimeEpisode {
            display_episode_number: 1,
            canonical_episode_number: 1,
            season_number: 1,
            title: Some("Movie".to_string()),
            overview: None,
            runtime_minutes: input.tmdb_episodes.first().and_then(|it| it.runtime_minutes),
            air_date: None,
            tmdb_reference: None,
            provider_reference: input.provider.map(|provider| ProviderEpisodeRef {
                provider,
                provider_show_id: None,
                provider_episode_token: None,
                provider_season_number: Some(1),
                provider_episode_number: Some("1".to_string()),
            }),
        };

        MappingOutput {
            seasons: vec![AnimeSeason {
                season_number: 1,
                title: "Movie".to_string(),
                episode_count: Some(1),
                source_kind: AnimeSeasonSourceKind::Hybrid,
                episodes: vec![episode],
            }],
            diagnostics: MappingDiagnostics {
                source_kind: AnimeSeasonSourceKind::Hybrid,
                fallback_used: false,
                notes: vec!["movie mapping path selected".to_string()],
            },
        }
    }

    fn map_series(&self, input: MappingInput) -> MappingOutput {
        if input.tmdb_episodes.is_empty()
            && input.anilist_episode_count.is_none()
            && input.released_episode_count.is_none()
        {
            return MappingOutput {
                seasons: vec![AnimeSeason {
                    season_number: 1,
                    title: "Season 1".to_string(),
                    episode_count: Some(1),
                    source_kind: AnimeSeasonSourceKind::ProviderDerived,
                    episodes: vec![AnimeEpisode {
                        display_episode_number: 1,
                        canonical_episode_number: 1,
                        season_number: 1,
                        title: Some("Episode 1".to_string()),
                        overview: None,
                        runtime_minutes: None,
                        air_date: None,
                        tmdb_reference: None,
                        provider_reference: None,
                    }],
                }],
                diagnostics: MappingDiagnostics {
                    source_kind: AnimeSeasonSourceKind::ProviderDerived,
                    fallback_used: true,
                    notes: vec!["missing episode metadata fallback".to_string()],
                },
            };
        }

        let mut episodes = Vec::new();
        if !input.tmdb_episodes.is_empty() {
            for (index, item) in input.tmdb_episodes.iter().enumerate() {
                let canonical = item
                    .anilist_episode_number
                    .unwrap_or((index + 1) as i32);
                let season_number = item.tmdb_season_number.unwrap_or(1);
                let display = item.tmdb_episode_number.unwrap_or(canonical);
                episodes.push(AnimeEpisode {
                    display_episode_number: display,
                    canonical_episode_number: canonical,
                    season_number,
                    title: item.title.clone().or_else(|| Some(format!("Episode {display}"))),
                    overview: None,
                    runtime_minutes: item.runtime_minutes,
                    air_date: None,
                    tmdb_reference: item
                        .tmdb_episode_number
                        .map(|episode| super::models::TmdbEpisodeRef {
                            season_number,
                            episode_number: episode,
                        }),
                    provider_reference: input.provider.clone().map(|provider| ProviderEpisodeRef {
                        provider,
                        provider_show_id: None,
                        provider_episode_token: None,
                        provider_season_number: Some(season_number),
                        provider_episode_number: Some(canonical.to_string()),
                    }),
                });
            }
        } else {
            let count = effective_episode_count(input.anilist_episode_count, input.released_episode_count);
            if let Some(count) = count {
                for episode_number in 1..=count {
                    episodes.push(AnimeEpisode {
                        display_episode_number: episode_number,
                        canonical_episode_number: episode_number,
                        season_number: 1,
                        title: Some(format!("Episode {episode_number}")),
                        overview: None,
                        runtime_minutes: None,
                        air_date: None,
                        tmdb_reference: None,
                        provider_reference: None,
                    });
                }
            }
        }

        let split_detected = episodes.iter().any(|ep| ep.season_number > 1);
        let source_kind = if split_detected {
            AnimeSeasonSourceKind::Hybrid
        } else if input.tmdb_episodes.is_empty() {
            AnimeSeasonSourceKind::Anilist
        } else {
            AnimeSeasonSourceKind::Tmdb
        };

        MappingOutput {
            seasons: vec![AnimeSeason {
                season_number: 1,
                title: if split_detected {
                    "Season 1 (virtualized)".to_string()
                } else {
                    "Season 1".to_string()
                },
                episode_count: Some(episodes.len() as i32),
                source_kind: source_kind.clone(),
                episodes,
            }],
            diagnostics: MappingDiagnostics {
                source_kind,
                fallback_used: false,
                notes: if split_detected {
                    vec!["split/virtual season behavior applied".to_string()]
                } else {
                    vec!["linear season mapping".to_string()]
                },
            },
        }
    }
}

fn effective_episode_count(anilist_episode_count: Option<i32>, released_episode_count: Option<i32>) -> Option<i32> {
    match (anilist_episode_count, released_episode_count) {
        (Some(total), Some(released)) => Some(total.min(released).max(0)),
        (Some(total), None) => Some(total.max(0)),
        (None, Some(released)) => Some(released.max(0)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_linear_season() {
        let service = AnimeMappingService::new();
        let output = service.map(MappingInput {
            is_movie: false,
            tmdb_episodes: vec![
                MappingEpisodeInput {
                    tmdb_season_number: Some(1),
                    tmdb_episode_number: Some(1),
                    anilist_episode_number: Some(1),
                    title: Some("One".to_string()),
                    runtime_minutes: Some(24),
                },
                MappingEpisodeInput {
                    tmdb_season_number: Some(1),
                    tmdb_episode_number: Some(2),
                    anilist_episode_number: Some(2),
                    title: Some("Two".to_string()),
                    runtime_minutes: Some(24),
                },
            ],
            anilist_episode_count: Some(2),
            released_episode_count: Some(2),
            provider: None,
        });

        assert_eq!(output.seasons[0].episodes.len(), 2);
        assert_eq!(output.diagnostics.notes[0], "linear season mapping");
    }

    #[test]
    fn applies_split_virtual_season_behavior() {
        let service = AnimeMappingService::new();
        let output = service.map(MappingInput {
            is_movie: false,
            tmdb_episodes: vec![
                MappingEpisodeInput {
                    tmdb_season_number: Some(1),
                    tmdb_episode_number: Some(12),
                    anilist_episode_number: Some(12),
                    title: None,
                    runtime_minutes: None,
                },
                MappingEpisodeInput {
                    tmdb_season_number: Some(2),
                    tmdb_episode_number: Some(1),
                    anilist_episode_number: Some(13),
                    title: None,
                    runtime_minutes: None,
                },
            ],
            anilist_episode_count: Some(24),
            released_episode_count: Some(24),
            provider: None,
        });

        assert!(output.seasons[0].title.contains("virtualized"));
        assert_eq!(output.diagnostics.notes[0], "split/virtual season behavior applied");
    }

    #[test]
    fn caps_anilist_generated_episodes_to_released_count() {
        let service = AnimeMappingService::new();
        let output = service.map(MappingInput {
            is_movie: false,
            tmdb_episodes: vec![],
            anilist_episode_count: Some(24),
            released_episode_count: Some(8),
            provider: None,
        });

        assert_eq!(output.seasons[0].episodes.len(), 8);
    }

    #[test]
    fn clamps_released_count_above_total() {
        let service = AnimeMappingService::new();
        let output = service.map(MappingInput {
            is_movie: false,
            tmdb_episodes: vec![],
            anilist_episode_count: Some(12),
            released_episode_count: Some(40),
            provider: None,
        });

        assert_eq!(output.seasons[0].episodes.len(), 12);
    }

    #[test]
    fn supports_zero_released_episodes() {
        let service = AnimeMappingService::new();
        let output = service.map(MappingInput {
            is_movie: false,
            tmdb_episodes: vec![],
            anilist_episode_count: Some(12),
            released_episode_count: Some(0),
            provider: None,
        });

        assert_eq!(output.seasons[0].episodes.len(), 0);
    }

    #[test]
    fn falls_back_when_episode_metadata_missing() {
        let service = AnimeMappingService::new();
        let output = service.map(MappingInput {
            is_movie: false,
            tmdb_episodes: vec![],
            anilist_episode_count: None,
            released_episode_count: None,
            provider: None,
        });

        assert!(output.diagnostics.fallback_used);
        assert_eq!(output.seasons[0].episodes.len(), 1);
    }
}
