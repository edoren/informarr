use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiscordConfig {
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub webhook_url: String,
    pub color: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SonarrConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RadarrConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeerrConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub seerr: SeerrConfig,
    pub discord: Option<DiscordConfig>,
    pub telegram: Option<TelegramConfig>,
    pub sonarr: Option<Vec<SonarrConfig>>,
    pub radarr: Option<Vec<RadarrConfig>>,
}
