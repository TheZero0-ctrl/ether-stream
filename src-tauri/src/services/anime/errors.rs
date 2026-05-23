use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimeErrorCategory {
    AnimeNotClassified,
    AnilistMatchMissing,
    SeasonMappingFailed,
    ProviderSearchFailed,
    ProviderEpisodeMissing,
    PlayableSourceMissing,
    TranslationUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeCommandError {
    pub category: AnimeErrorCategory,
    pub message: String,
    pub context: Option<String>,
}
