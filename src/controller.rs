use std::time::Duration;

use anyhow::{Result, anyhow};
use bon::Builder;
use log::{debug, error, info, warn};
use reqwest::Url;
use serde_json::{Value, json};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{
    sync::{mpsc, watch},
    time::Instant,
};

use jellyseerr::{
    apis::{
        Api as _, movies_api::MovieMovieIdGetParams, request_api::RequestRequestIdGetParams,
        tv_api::TvTvIdGetParams, users_api::UserUserIdGetParams,
    },
    models::{JellyseerrMediaRequest, JellyseerrMovieDetails, JellyseerrTvDetails},
};
use radarr::apis::Api as _;
use sonarr::{
    apis::{Api as _, series_api::ApiV3SeriesIdGetParams},
    models::SonarrSeasonResource,
};

use crate::{
    AppConfig, DiscordConfig, RadarrConfig, SonarrConfig, TelegramConfig,
    webhooks::{self, jellyseerr::JellyseerrEvent, radarr::RadarrEvent, sonarr::SonarrEvent},
};

// use thiserror::Error;

// #[derive(Error, Debug)]
// pub enum WawaError {
//     // #[error("data store disconnected")]
//     // Disconnect(#[from] io::Error),
//     #[error("execution failed: {0}")]
//     ExecutionFailed(String),
//     #[error("missing data error: {0}")]
//     MissingData(String),
//     #[error("yay: {0}")]
//     YOLO(String),
//     // #[error("invalid header (expected {expected:?}, found {found:?})")]
//     // InvalidHeader {
//     //     expected: String,
//     //     found: String,
//     // },
//     // #[error("unknown data store error")]
//     // Unknown,
// }

#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u64)]
enum EmbedColors {
    Default = 0,
    Aqua = 1752220,
    Green = 3066993,
    Blue = 3447003,
    Purple = 10181046,
    Gold = 15844367,
    Orange = 15105570,
    Red = 15158332,
    Grey = 9807270,
    DarkerGrey = 8359053,
    Navy = 3426654,
    DarkAqua = 1146986,
    DarkGreen = 2067276,
    DarkBlue = 2123412,
    DarkPurple = 7419530,
    DarkGold = 12745742,
    DarkOrange = 11027200,
    DarkRed = 10038562,
    DarkGrey = 9936031,
    LightGrey = 12370112,
    DarkNavy = 2899536,
    LuminousVividPink = 16580705,
    DarkVividPink = 12320855,
}

#[derive(Debug, Clone, PartialEq)]
enum NotificationType {
    MediaAvailable,
    OngoingSeasonAvailable,
    OngoingEpisodeAvailable,
}

#[derive(Debug, Clone, Builder)]
struct NotificationData {
    r#type: NotificationType,
    media_request: MediaRequest,
    seasons: Option<Vec<i32>>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
}

struct NotificationController {
    discord: Option<DiscordConfig>,
    telegram: Option<TelegramConfig>,
}

impl NotificationController {
    fn new(discord: Option<DiscordConfig>, telegram: Option<TelegramConfig>) -> Self {
        Self { discord, telegram }
    }

    async fn send_notification(&self, notification_request: NotificationData) {
        if let Err(e) = self.send_discord_notification(&notification_request).await {
            warn!("Failed sending Discord message: {e}");
        }

        if let Err(e) = self.send_telegram_notification(&notification_request).await {
            warn!("Failed sending Telegram message: {e}");
        }
    }

    async fn send_telegram_notification(&self, data: &NotificationData) -> Result<()> {
        let telegram = match self.telegram {
            Some(ref config) => config,
            None => return Ok(()),
        };

        let media_request = &data.media_request;

        // Create a multipart form to send the photo
        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", telegram.chat_id.clone())
            .text("parse_mode", "HTML"); // Add the caption text

        let prefix_text;
        let title;
        let message;
        let mut is_photo = false;
        match data.r#type {
            NotificationType::MediaAvailable => {
                let seasons_joined = data
                    .seasons
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                if let Some(url) = &media_request.image_url {
                    form = form.text("photo", url.to_string());
                    is_photo = true;
                }
                prefix_text = "New Content Available";
                let season_string = format!("Season {seasons_joined}");
                title = if seasons_joined.is_empty() {
                    media_request.media.title.clone()
                } else {
                    format!("{} - {}", media_request.media.title, season_string)
                };
                message = media_request.media.overview.clone();
            }
            NotificationType::OngoingSeasonAvailable => {
                prefix_text = "New Content Available";
                let season_string = data
                    .season_number
                    .map_or(String::new(), |s| format!("Season {s}"));
                let episode_string = data
                    .episode_number
                    .map_or(String::new(), |e| format!("Episode 1 to {e}"));
                title = format!(
                    "{} - {} {}",
                    media_request.media.title, season_string, episode_string
                );
                message = media_request.media.overview.clone();
            }
            NotificationType::OngoingEpisodeAvailable => {
                prefix_text = "New Episode Available";
                let season_string = data
                    .season_number
                    .map_or(String::new(), |s| format!("Season {s}"));
                let episode_string = data
                    .episode_number
                    .map_or(String::new(), |e| format!("Episode {e}"));
                title = format!(
                    "{} - {} {}",
                    media_request.media.title, season_string, episode_string
                );
                message = String::new();
            }
        };

        form = form.text(
            if is_photo { "caption" } else { "text" },
            format!("<b>{prefix_text}</b>\n\n<b>{title}</b>\n\n{message}")
                .trim()
                .to_string(),
        );

        self.send_telegram_request(form, is_photo).await?;

        Ok(())
    }

    async fn send_telegram_request(
        &self,
        form: reqwest::multipart::Form,
        is_photo: bool,
    ) -> Result<()> {
        let bot_token = match self.telegram {
            Some(ref config) => &config.bot_token,
            None => return Ok(()),
        };

        // Create a reqwest client
        let client = reqwest::Client::new();

        // Send the POST request to the Telegram Bot API
        let url = if is_photo {
            format!("https://api.telegram.org/bot{}/sendPhoto", bot_token)
        } else {
            format!("https://api.telegram.org/bot{}/sendMessage", bot_token)
        };
        let response = client.post(&url).multipart(form).send().await?;

        // Check if the request was successful
        if !response.status().is_success() {
            error!(
                "Failed to send photo: {:?} {:?}",
                response.status(),
                response.text().await
            );
        }

        Ok(())
    }

    async fn send_discord_notification(&self, data: &NotificationData) -> Result<()> {
        let discord = match self.discord {
            Some(ref config) => config,
            None => return Ok(()),
        };

        let media_request = &data.media_request;

        let color = EmbedColors::Green;
        let mut fields = Vec::new();

        fields.push(json!({
            "name": "Requested By",
            "value": media_request.requested_by.display_name,
            "inline": true,
        }));

        // let status;
        // (color, status) = match payload.notification_type {
        //     jellyseerr::NotificationType::MediaPending => (EmbedColors::Orange, "Pending Approval"),
        //     jellyseerr::NotificationType::MediaApproved
        //     | jellyseerr::NotificationType::MediaAutoApproved => {
        //         (EmbedColors::Purple, "Processing")
        //     }
        //     jellyseerr::NotificationType::MediaAvailable => (EmbedColors::Green, "Available"),
        //     jellyseerr::NotificationType::MediaDeclined => (EmbedColors::Red, "Declined"),
        //     jellyseerr::NotificationType::MediaFailed => (EmbedColors::Red, "Failed"),
        //     _ => (color, ""),
        // };

        // fields.push(json!({
        //     "name": "Request Status",
        //     "value": "Available",
        //     "inline": true,
        // }));

        // if let Some(comment) = &payload.comment {
        //     fields.push(json!({
        //         "name": format!("Comment from {}", comment.commented_by_username),
        //         "value": comment.comment_message,
        //         "inline": false,
        //     }));
        // } else if let Some(issue) = &payload.issue {
        //     fields.push(json!([
        //         {
        //             "name": "Reported By",
        //             "value": issue.reported_by_username,
        //             "inline": true,
        //         },
        //         {
        //             "name": "Issue Type",
        //             "value": issue.issue_type.name(),
        //             "inline": true,
        //         },
        //         {
        //             "name": "Issue Status",
        //             "value": if issue.issue_status == jellyseerr::IssueStatus::Open { "Open" } else { "Resolved" },
        //             "inline": true,
        //         }
        //     ]));

        //     color = match payload.notification_type {
        //         jellyseerr::NotificationType::IssueCreated
        //         | jellyseerr::NotificationType::IssueReopened => EmbedColors::Red,
        //         jellyseerr::NotificationType::IssueComment => EmbedColors::Orange,
        //         jellyseerr::NotificationType::IssueResolved => EmbedColors::Green,
        //         _ => color,
        //     };
        // }

        let pre_title;
        let title;
        let description;

        let image_url = media_request
            .image_url
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();

        match data.r#type {
            NotificationType::MediaAvailable => {
                pre_title = "New Content Available".to_string();
                title = media_request.media.title.clone();
                description = media_request.media.overview.clone();

                if media_request.r#type == MediaType::TV && data.seasons.is_some() {
                    if let Some(seasons) = &data.seasons {
                        let seasons_joined = seasons
                            .iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<String>>()
                            .join(", ");
                        fields.push(json!({
                          "name": "Seasons",
                          "value": seasons_joined,
                          "inline": true,
                        }));
                    }
                }
            }
            NotificationType::OngoingSeasonAvailable => {
                pre_title = "New Content Available".to_string();
                title = media_request.media.title.clone();
                description = media_request.media.overview.clone();

                let season_string = data.season_number.map_or(String::new(), |s| format!("{s}"));
                let episode_string = data
                    .episode_number
                    .map_or(String::new(), |e| format!("1 to {e}"));

                fields.push(json!({
                  "name": "Season",
                  "value": season_string,
                  "inline": true,
                }));
                fields.push(json!({
                  "name": "Episode",
                  "value": episode_string,
                  "inline": true,
                }))
            }
            NotificationType::OngoingEpisodeAvailable => {
                pre_title = "New Episode Now Available".to_string();
                let season_string = data
                    .season_number
                    .map_or(String::new(), |s| format!("Season {s}"));
                let episode_string = data
                    .episode_number
                    .map_or(String::new(), |e| format!("Episode {e}"));

                title = media_request.media.title.clone();
                description = String::new();

                fields.push(json!({
                  "name": "Season",
                  "value": season_string,
                  "inline": true,
                }));
                fields.push(json!({
                  "name": "Episode",
                  "value": episode_string,
                  "inline": true,
                }))
            }
        }

        let content = match &media_request.requested_by.discord_id {
            Some(val) => format!("<@{val}>"),
            None => String::new(),
        };

        let message = json!({
            "username": discord.username,
            "avatar_url": discord.avatar_url,
            "content": content,
            "embeds": [
                {
                    "title": title,
                    "description": description,
                    "color": color,
                    "timestamp": OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default(),
                    "author": {
                        "name": pre_title,
                        "url": image_url,
                    },
                    "fields": fields,
                    "thumbnail": {
                        "url": image_url,
                    },
                }
            ]
        });

        self.send_discord_request(message).await?;

        Ok(())
    }

    async fn send_discord_request(&self, data: Value) -> Result<()> {
        let webhook_url = match self.discord {
            Some(ref config) => &config.webhook_url,
            None => return Ok(()),
        };

        // Create a reqwest client
        let client = reqwest::Client::new();

        // Send the message via the Webhook URL using a POST request
        let response = client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        // Check if the request was successful
        if !response.status().is_success() {
            error!(
                "Failed to send message: {:?} {:?}",
                response.status(),
                response.json::<Value>().await
            );
        }

        Ok(())
    }
}

struct RequestHandler {
    jellyseerr_api: jellyseerr::apis::ApiClient,
    sonarr_apis: Vec<sonarr::apis::ApiClient>,
    // radarr_apis: Vec<radarr::apis::ApiClient>,
    requested: Vec<MediaRequest>,
    // users: HashMap<i32, Arc<JellyseerrUser>>,
    notifier: NotificationController,
}

impl RequestHandler {
    async fn new(mut app_config: AppConfig) -> Result<Self> {
        let config = jellyseerr::apis::configuration::Configuration {
            base_path: format!("{}/{}", app_config.url, "api/v1"),
            api_key: Some(jellyseerr::apis::configuration::ApiKey {
                prefix: None,
                key: app_config.api_key,
            }),
            ..Default::default()
        };
        app_config.url.clear();
        let jellyseerr_api = jellyseerr::apis::ApiClient::new(config.into());

        let convert_to_url = |use_ssl, host, port| {
            format!(
                "{}://{}:{}",
                if use_ssl { "https" } else { "http" },
                host,
                port
            )
        };

        let sonarr_settings = if let Some(config) = app_config.sonarr.take() {
            config
        } else {
            jellyseerr_api
                .settings_api()
                .settings_sonarr_get()
                .await
                .map_err(|e| anyhow!("Could not retrieve Sonarr settings {e}"))?
                .into_iter()
                .map(|s| SonarrConfig {
                    url: convert_to_url(s.use_ssl, s.hostname, s.port),
                    api_key: s.api_key,
                })
                .collect()
        };
        let radarr_settings = if let Some(config) = app_config.radarr.take() {
            config
        } else {
            jellyseerr_api
                .settings_api()
                .settings_radarr_get()
                .await
                .map_err(|e| anyhow!("Could not retrieve Radarr settings {e}"))?
                .into_iter()
                .map(|s| RadarrConfig {
                    url: convert_to_url(s.use_ssl, s.hostname, s.port),
                    api_key: s.api_key,
                })
                .collect()
        };

        let sonarr_apis = sonarr_settings
            .into_iter()
            .map(|settings| {
                let config = sonarr::apis::configuration::Configuration {
                    base_path: settings.url,
                    api_key: Some(sonarr::apis::configuration::ApiKey {
                        prefix: None,
                        key: settings.api_key,
                    }),
                    ..Default::default()
                };
                sonarr::apis::ApiClient::new(config.into())
            })
            .collect::<Vec<sonarr::apis::ApiClient>>();

        let radarr_apis = radarr_settings
            .into_iter()
            .map(|settings| {
                let config = radarr::apis::configuration::Configuration {
                    base_path: settings.url,
                    api_key: Some(radarr::apis::configuration::ApiKey {
                        prefix: None,
                        key: settings.api_key,
                    }),
                    ..Default::default()
                };
                radarr::apis::ApiClient::new(config.into())
            })
            .collect::<Vec<radarr::apis::ApiClient>>();

        debug!("Sonarr instances: {}", sonarr_apis.len());
        debug!("Radarr instances: {}", radarr_apis.len());

        for instance in &sonarr_apis {
            instance
                .api_info_api()
                .api_get()
                .await
                .map_err(|e| anyhow!("Could not retrieve Sonarr API info {e}"))?;
        }

        for instance in &radarr_apis {
            instance
                .api_info_api()
                .api_get()
                .await
                .map_err(|e| anyhow!("Could not retrieve Radarr API info {e}"))?;
        }

        let mut instance = Self {
            jellyseerr_api,
            sonarr_apis,
            // radarr_apis,
            requested: Vec::new(),
            // users: HashMap::new(),
            notifier: NotificationController::new(app_config.discord, app_config.telegram),
        };

        instance
            .fetch_requests()
            .await
            .map_err(|e| anyhow!("Could not fetch initial requests: {e}"))?;

        Ok(instance)
    }

    async fn process_jellyseerr(&mut self, event: JellyseerrEvent) -> Result<()> {
        if event.notification_type == webhooks::jellyseerr::NotificationType::MediaApproved
            || event.notification_type == webhooks::jellyseerr::NotificationType::MediaAutoApproved
        {
            let request_id = match event.request {
                Some(request) => request.request_id,
                None => return Ok(()),
            };

            if let Ok(media_request) = self
                .jellyseerr_api
                .request_api()
                .request_request_id_get(
                    RequestRequestIdGetParams::builder()
                        .request_id(request_id)
                        .build(),
                )
                .await
            {
                self.process_request(media_request).await?;
            }
        }

        Ok(())
    }

    async fn process_sonarr(&mut self, event: SonarrEvent) -> Result<()> {
        let download_event = match event {
            SonarrEvent::Download(event) => event,
            _ => return Ok(()),
        };

        let is_import_completed_event = download_event.episode_files.is_some();
        let is_upgrade = download_event.is_upgrade;
        if is_import_completed_event || is_upgrade {
            debug!("Is import complete or upgrade, skipping");
            return Ok(());
        }

        let requested_show = self
            .get_tv_request(
                download_event.series.tmdb_id,
                Some(download_event.series.tvdb_id),
            )
            .cloned()
            .ok_or(anyhow!(
                "Could not find requested show with tmdb id {}",
                download_event.series.tmdb_id
            ))?;

        let requested_seasons = requested_show.seasons.as_ref().ok_or(anyhow!(
            "Could not find seasons for requested show with tmdb id {}",
            download_event.series.tmdb_id
        ))?;

        let event_seasons: Vec<i32> = download_event
            .episodes
            .iter()
            .map(|episode| episode.season_number)
            .collect();

        let requested_seasons_with_event: Vec<&SeasonInfo> = requested_seasons
            .iter()
            .filter(|season| event_seasons.contains(&season.season_number))
            .collect();

        if requested_seasons_with_event.is_empty() {
            debug!("No seasons to update");
            return Ok(());
        }

        let sonarr_series = {
            let mut series = None;
            for api in &self.sonarr_apis {
                if let Ok(res) = api
                    .series_api()
                    .api_v3_series_id_get(
                        ApiV3SeriesIdGetParams::builder()
                            .id(download_event.series.id)
                            .build(),
                    )
                    .await
                {
                    series = Some(res);
                    break;
                }
            }
            series.ok_or(anyhow!(
                "Could not find Sonarr series with id {}",
                download_event.series.id
            ))?
        };

        let sonarr_monitored_seasons = sonarr_series
            .seasons
            .unwrap_or_default()
            .ok_or(anyhow!(
                "Could not find seasons for the specified Sonarr series"
            ))?
            .into_iter()
            .filter(|season| season.monitored.unwrap_or(false))
            .collect::<Vec<_>>();

        if sonarr_monitored_seasons.is_empty() {
            return Err(anyhow!("Could not find monitored Sonarr series seasons"));
        }

        let mut sonarr_available_seasons: Vec<&SonarrSeasonResource> = Vec::new();
        let mut sonarr_completed_seasons = Vec::new();
        let mut sonarr_ongoing_seasons = Vec::new();
        for season in &sonarr_monitored_seasons {
            if let Some(stats) = &season.statistics {
                let is_season_ongoing = stats.next_airing.is_some();
                if is_season_ongoing {
                    sonarr_ongoing_seasons.push(season);
                } else {
                    sonarr_completed_seasons.push(season);
                    if stats
                        .episode_file_count
                        .is_some_and(|v| Some(v) == stats.total_episode_count)
                    {
                        sonarr_available_seasons.push(season);
                    }
                }
            }
        }

        let requested_matching_count = |seasons: &Vec<&SonarrSeasonResource>| {
            requested_seasons_with_event
                .iter()
                .filter(|requested_season| {
                    seasons
                        .iter()
                        .find(|sonarr_season| {
                            sonarr_season
                                .season_number
                                .is_some_and(|n| n == requested_season.season_number)
                        })
                        .is_some()
                })
                .map(|r| *r)
                .collect::<Vec<&SeasonInfo>>()
        };

        let available_requested = requested_matching_count(&sonarr_available_seasons);
        let completed_requested = requested_matching_count(&sonarr_completed_seasons);

        // SEASONS REQUESTED AVALIABLE NOTIFICATION
        if completed_requested.len() != 0 && completed_requested.len() == available_requested.len()
        {
            info!("Sending notification for request available");
            self.notifier
                .send_notification(
                    NotificationData::builder()
                        .r#type(NotificationType::MediaAvailable)
                        .media_request(requested_show.clone())
                        .seasons(
                            available_requested
                                .iter()
                                .map(|season| season.season_number)
                                .collect(),
                        )
                        .build(),
                )
                .await;
            self.remove_request(&requested_show);
            return Ok(());
        }

        // ONGOING SEASON NOTIFICATION
        let ongoing_requested = requested_matching_count(&sonarr_ongoing_seasons);
        if ongoing_requested.is_empty() {
            debug!("No ongoing seasons to update");
            return Ok(());
        }

        let sonarr_first_ongoing_season = sonarr_ongoing_seasons
            .iter()
            .min_by(|a, b| a.season_number.cmp(&b.season_number))
            .map(|v| *v)
            .ok_or(anyhow!("Could not get first ongoing season"))?;
        let first_requested_ongoing_season = ongoing_requested
            .iter()
            .min_by(|a, b| a.season_number.cmp(&b.season_number))
            .map(|v| *v)
            .ok_or(anyhow!("Could not get first requested ongoing season"))?;

        // Only send notifications when we confirm that the latest episode of the ongoing season has been downloaded
        let should_send_notification = sonarr_first_ongoing_season
            .statistics
            .as_ref()
            .is_some_and(|stats| stats.episode_file_count != stats.episode_count)
            && (sonarr_first_ongoing_season.season_number
                == Some(first_requested_ongoing_season.season_number));
        if should_send_notification {
            debug!("Missing episodes to send notification");
            return Ok(());
        }

        let last_episode_season = sonarr_first_ongoing_season
            .season_number
            .ok_or(anyhow!("Could not get last episode season"))?;
        let last_episode_number = sonarr_first_ongoing_season
            .statistics
            .as_ref()
            .and_then(|stats| stats.episode_count)
            .ok_or(anyhow!("Could not get last episode number"))?;
        let last_episode_air_date = sonarr_first_ongoing_season
            .statistics
            .as_ref()
            .and_then(|stats| stats.previous_airing.as_ref())
            .and_then(|prev_airing_opt| prev_airing_opt.as_deref())
            .and_then(|prev_airing| OffsetDateTime::parse(&prev_airing, &Rfc3339).ok())
            .ok_or(anyhow!("Could not get last episode air date"))?;

        if last_episode_air_date > requested_show.created_at || last_episode_number == 1 {
            info!("Sending notification for single episode available");
            self.notifier
                .send_notification(
                    NotificationData::builder()
                        .r#type(NotificationType::OngoingEpisodeAvailable)
                        .media_request(requested_show)
                        .season_number(last_episode_season)
                        .episode_number(last_episode_number)
                        .build(),
                )
                .await;
        } else {
            info!("Sending notification for multiple ongoing episodes available");
            self.notifier
                .send_notification(
                    NotificationData::builder()
                        .r#type(NotificationType::OngoingSeasonAvailable)
                        .media_request(requested_show)
                        .season_number(last_episode_season)
                        .episode_number(last_episode_number)
                        .build(),
                )
                .await;
        }

        return Ok(());
    }

    async fn process_radarr(&mut self, event: RadarrEvent) -> Result<()> {
        let download_event = match event {
            RadarrEvent::Download(event) => event,
            _ => return Ok(()),
        };

        if let Some(requested_movie) = self
            .get_movie_request(download_event.movie.tmdb_id)
            .cloned()
        {
            self.notifier
                .send_notification(
                    NotificationData::builder()
                        .r#type(NotificationType::MediaAvailable)
                        .media_request(requested_movie.clone())
                        .build(),
                )
                .await;
            self.remove_request(&requested_movie);
        }

        Ok(())
    }

    fn get_tv_request(&self, tmdb_id: i32, tvdb_id: Option<i32>) -> Option<&MediaRequest> {
        let mut requested_shows = self
            .requested
            .iter()
            .filter(|request| request.r#type == MediaType::TV);

        requested_shows.find(|request| {
            request.media.tmdb_id == tmdb_id
                && request.media.tvdb_id.is_some_and(|id| Some(id) == tvdb_id)
        })
    }

    fn get_movie_request(&self, tmdb_id: i32) -> Option<&MediaRequest> {
        let mut requested_movies = self
            .requested
            .iter()
            .filter(|request| request.r#type == MediaType::MOVIE);

        requested_movies.find(|request| request.media.tmdb_id == tmdb_id)
    }

    fn remove_request(&mut self, media_request: &MediaRequest) {
        if let Some(pos) = self.requested.iter().position(|stored_request| {
            self.get_media_same(&stored_request.media, &media_request.media)
        }) {
            self.requested.remove(pos);
        }
    }

    fn get_media_same(&self, media_1: &MediaInfo, media_2: &MediaInfo) -> bool {
        media_1.tmdb_id == media_2.tmdb_id
            || media_1.tvdb_id.is_some_and(|v| Some(v) == media_2.tvdb_id)
    }

    async fn get_movie_by_id(&self, tmdb_id: i32) -> Result<JellyseerrMovieDetails> {
        self.jellyseerr_api
            .movies_api()
            .movie_movie_id_get(
                MovieMovieIdGetParams::builder()
                    .movie_id(tmdb_id as f64)
                    .build(),
            )
            .await
            .map_err(|e| anyhow!("Could not get movie with id {tmdb_id}: {e}"))
    }

    async fn get_show_by_id(&self, tmdb_id: i32) -> Result<JellyseerrTvDetails> {
        self.jellyseerr_api
            .tv_api()
            .tv_tv_id_get(TvTvIdGetParams::builder().tv_id(tmdb_id as f64).build())
            .await
            .map_err(|e| anyhow!("Could not get show with id {tmdb_id}: {e}"))
    }

    async fn fetch_requests(&mut self) -> Result<()> {
        self.requested.clear();

        let num_requests = self
            .jellyseerr_api
            .request_api()
            .request_count_get()
            .await?
            .approved
            .unwrap_or(0.0) as usize;

        let batch_size = 100;
        let mut offset = 0;

        while offset < num_requests {
            let requests = self
                .jellyseerr_api
                .request_api()
                .request_get(
                    jellyseerr::apis::request_api::RequestGetParams::builder()
                        .take(batch_size as f64)
                        .skip(offset as f64)
                        .filter("approved".into())
                        .build(),
                )
                .await?
                .results
                .unwrap_or_default();

            for media_request in requests {
                self.process_request(media_request).await?;
            }

            offset += batch_size;
        }

        Ok(())
    }

    async fn process_request(&mut self, media_request: JellyseerrMediaRequest) -> Result<()> {
        let media_type = media_request
            .r#type
            .as_ref()
            .ok_or(anyhow!("Could not get media type"))?;
        let media = media_request
            .media
            .as_ref()
            .ok_or(anyhow!("Could not get media"))?;

        // if AVAILABLE
        if media.status.unwrap_or(0) == 5 {
            return Ok(());
        }

        let tmdb_id = media
            .tmdb_id
            .ok_or(anyhow!("Could not get media tmdb_id"))?;
        let tvdb_id = media.tvdb_id.unwrap_or_default();

        let created_at = media_request
            .created_at
            .as_deref()
            .and_then(|date| OffsetDateTime::parse(&date, &Rfc3339).ok())
            .ok_or(anyhow!("Could not get request creation time"))?;

        let user_id = media_request
            .requested_by
            .as_ref()
            .map(|user| user.id)
            .ok_or(anyhow!("Could not get user id"))?;

        let user = self
            .jellyseerr_api
            .users_api()
            .user_user_id_get(
                UserUserIdGetParams::builder()
                    .user_id(user_id as f64)
                    .build(),
            )
            .await
            .map_err(|e| anyhow!("Could not get user with id {user_id}: {e}"))?;
        // let username = user
        //     .username
        //     .clone()
        //     .unwrap_or_default()
        //     .or_else(|| user.jellyfin_username.clone().unwrap_or_default())
        //     .or_else(|| user.plex_username.clone().unwrap_or_default())
        //     .ok_or(anyhow!("username not set"))?;
        let display_name = user
            .display_name
            .clone()
            .or_else(|| user.username.clone().unwrap_or_default())
            .or_else(|| user.jellyfin_username.clone().unwrap_or_default())
            .or_else(|| user.plex_username.clone().unwrap_or_default())
            .ok_or(anyhow!("Display name not set"))?;
        let discord_id = user
            .settings
            .and_then(|settings| settings.discord_id.unwrap_or_default());

        // let watch_url = media.media_url.as_ref().and_then(|v| Url::parse(v).ok());

        if media_type == "tv" {
            let show = self
                .get_show_by_id(tmdb_id)
                .await
                .map_err(|e| anyhow!("Could not get show from Jellyseerr: {e}"))?;

            let requested_seasons = media_request
                .seasons
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter_map(|s| {
                    s.season_number.map(|n| SeasonInfo {
                        season_number: n as i32,
                    })
                })
                .collect::<Vec<SeasonInfo>>();

            let available_seasons = show
                .media_info
                .as_ref()
                .and_then(|info| info.seasons.as_deref())
                .unwrap_or_default()
                .iter()
                .filter_map(|s| {
                    if s.status == Some(5) {
                        s.season_number.map(|n| n as i32)
                    } else {
                        None
                    }
                })
                .collect::<Vec<i32>>();

            let seasons_missing = requested_seasons
                .iter()
                .filter(|s| !available_seasons.contains(&s.season_number))
                .collect::<Vec<&SeasonInfo>>();

            if !seasons_missing.is_empty() {
                let processed_request = MediaRequest {
                    r#type: MediaType::TV,
                    media: MediaInfo {
                        tmdb_id: tmdb_id,
                        tvdb_id: tvdb_id,
                        title: show.name.or_else(|| show.original_name).unwrap_or_default(),
                        overview: show.overview.unwrap_or_default(),
                    },
                    created_at: created_at,
                    requested_by: User {
                        display_name: display_name,
                        discord_id: discord_id,
                    },
                    image_url: show.poster_path.and_then(|path| {
                        Url::parse(&format!(
                            "https://image.tmdb.org/t/p/w600_and_h900_bestv2{}",
                            path
                        ))
                        .ok()
                    }),
                    // watch_url: watch_url,
                    seasons: Some(requested_seasons),
                };

                debug!("Request added: {:?}", processed_request);
                self.requested.push(processed_request);
            }
        } else if media_type == "movie" {
            let movie = self
                .get_movie_by_id(tmdb_id)
                .await
                .map_err(|e| anyhow!("Could not get show from Jellyseerr: {e}"))?;

            let processed_request = MediaRequest {
                r#type: MediaType::MOVIE,
                media: MediaInfo {
                    tmdb_id: tmdb_id,
                    tvdb_id: tvdb_id,
                    title: movie
                        .title
                        .or_else(|| movie.original_title)
                        .unwrap_or_default(),
                    overview: movie.overview.unwrap_or_default(),
                },
                created_at: created_at,
                requested_by: User {
                    display_name: display_name,
                    discord_id: discord_id,
                },
                image_url: movie.poster_path.and_then(|path| {
                    Url::parse(&format!(
                        "https://image.tmdb.org/t/p/w600_and_h900_bestv2{}",
                        path
                    ))
                    .ok()
                }),
                // watch_url: watch_url,
                seasons: None,
            };

            debug!("Request added: {:?}", processed_request);
            self.requested.push(processed_request);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum MediaType {
    MOVIE,
    TV,
}

#[derive(Debug, Clone)]
struct SeasonInfo {
    season_number: i32,
}

#[derive(Debug, Clone)]
struct MediaInfo {
    tmdb_id: i32,
    tvdb_id: Option<i32>,
    title: String,
    overview: String,
}

#[derive(Debug, Clone)]
struct User {
    display_name: String,
    discord_id: Option<String>,
}

#[derive(Debug, Clone)]
struct MediaRequest {
    r#type: MediaType,
    media: MediaInfo,
    created_at: OffsetDateTime,
    requested_by: User,
    image_url: Option<Url>,
    seasons: Option<Vec<SeasonInfo>>,
}

pub async fn run(
    app_config: AppConfig,
    mut sonarr_rx: mpsc::UnboundedReceiver<SonarrEvent>,
    mut radarr_rx: mpsc::UnboundedReceiver<RadarrEvent>,
    mut jellyseerr_rx: mpsc::UnboundedReceiver<JellyseerrEvent>,
    close_tx: watch::Sender<bool>,
    mut close_rx: watch::Receiver<bool>,
) -> Result<()> {
    let mut request_handler = match RequestHandler::new(app_config).await {
        Ok(handler) => handler,
        Err(e) => {
            let _ = close_tx.send(true);
            return Err(anyhow!("Failed to initialize request handler: {e}"));
        }
    };

    let scan_interval = Duration::from_secs(60 * 30);
    let mut next_scan_requests = Instant::now() + scan_interval;

    loop {
        tokio::select! {
            Some(event) = sonarr_rx.recv() => {
                info!("Processing Sonarr event: {}", event);
                debug!("Received from Sonarr: {}", serde_json::to_string(&event).unwrap_or_default());
                if let Err(e) = request_handler.process_sonarr(event).await {
                    error!("Failed processing Sonarr event: {e}");
                }
            },
            Some(event) = radarr_rx.recv() => {
                info!("Processing Radarr event: {}", event);
                debug!("Received from Radarr: {}", serde_json::to_string(&event).unwrap_or_default());
                if let Err(e) = request_handler.process_radarr(event).await {
                    error!("Failed processing Radarr event: {e}");
                }
            },
            Some(event) = jellyseerr_rx.recv() => {
                info!("Processing Jellyseerr event: {}", event.notification_type);
                debug!("Received from Jellyseerr: {}", serde_json::to_string(&event).unwrap_or_default());
                if let Err(e) = request_handler.process_jellyseerr(event).await {
                    error!("Failed processing Jellyseerr event: {e}");
                }
            }
            _ = tokio::time::sleep_until(next_scan_requests.into()) => {
                if let Err(e) = request_handler.fetch_requests().await {
                    error!("Failed to update requests: {e}");
                }
                next_scan_requests = Instant::now() + scan_interval;
            }
            result = close_rx.changed() => {
                debug!("Closing controller");
                if result.is_ok() && *close_rx.borrow_and_update() {
                    break Ok(());
                }
            }
        };
    }
}
