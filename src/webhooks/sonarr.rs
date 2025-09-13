use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::MessageResponse;

pub const TAG: &str = "sonarr";

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeriesImage {
    pub cover_type: String,
    pub remote_url: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub genres: Option<Vec<String>>,
    pub id: i32,
    pub images: Option<Vec<SeriesImage>>,
    pub imdb_id: Option<String>,
    pub original_language: Option<Language>,
    pub path: String,
    pub tags: Option<Vec<String>>,
    pub title: String,
    pub title_slug: Option<String>,
    pub tmdb_id: i32,
    pub tv_maze_id: i32,
    pub tvdb_id: i32,
    pub r#type: String,
    pub year: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub episode_number: i32,
    pub id: i32,
    pub season_number: i32,
    pub series_id: i32,
    pub title: String,
    pub tvdb_id: i32,
    pub air_date: Option<String>,
    pub air_date_utc: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormat {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormatInfo {
    pub custom_format_score: i32,
    pub custom_formats: Vec<CustomFormat>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadClientItem {
    pub quality: String,
    pub quality_version: i32,
    pub size: i64,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatusMessage {
    pub title: String,
    pub messages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub custom_format_score: Option<i32>,
    pub custom_formats: Option<Vec<String>>,
    pub indexer: Option<String>,
    pub indexer_flags: Option<Vec<String>>,
    pub languages: Option<Vec<Language>>,
    pub quality: Option<String>,
    pub quality_version: Option<i32>,
    pub release_group: Option<String>,
    pub release_title: Option<String>,
    pub size: Option<i64>,
    pub release_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub audio_channels: f32,
    pub audio_codec: String,
    pub audio_languages: Vec<String>,
    pub height: i32,
    pub subtitles: Vec<String>,
    pub video_codec: String,
    pub video_dynamic_range: String,
    pub video_dynamic_range_type: String,
    pub width: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFile {
    pub date_added: String,
    pub id: i32,
    pub languages: Vec<Language>,
    pub media_info: MediaInfo,
    pub path: String,
    pub quality: Option<String>,
    pub quality_version: Option<i32>,
    pub relative_path: Option<String>,
    pub release_group: Option<String>,
    pub scene_name: Option<String>,
    pub size: Option<i64>,
    pub source_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenamedEpisodeFile {
    pub previous_relative_path: String,
    pub previous_path: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestEvent {
    pub application_url: String,
    pub instance_name: String,
    pub series: Series,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrabEvent {
    pub application_url: String,
    pub instance_name: String,
    pub custom_format_info: CustomFormatInfo,
    pub download_client: String,
    pub download_client_type: String,
    pub download_id: String,
    pub series: Series,
    pub episodes: Vec<Episode>,
    pub release: Release,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub application_url: String,
    pub custom_format_info: Option<CustomFormatInfo>,
    pub deleted_files: Option<Vec<EpisodeFile>>,
    pub destination_path: Option<String>,
    pub download_client_type: Option<String>,
    pub download_client: Option<String>,
    pub download_id: Option<String>,
    pub episode_file: Option<EpisodeFile>,
    pub episode_files: Option<Vec<EpisodeFile>>,
    pub episodes: Vec<Episode>,
    pub file_count: Option<i32>,
    pub instance_name: String,
    #[serde(default)]
    pub is_upgrade: bool,
    pub release: Release,
    pub series: Series,
    pub source_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFileDeleteEvent {
    pub application_url: String,
    pub delete_reason: String,
    pub episode_file: EpisodeFile,
    pub episodes: Vec<Episode>,
    pub instance_name: String,
    pub series: Series,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeriesAddEvent {
    pub application_url: String,
    pub instance_name: String,
    pub series: Series,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDeleteEvent {
    pub application_url: String,
    pub instance_name: String,
    pub series: Series,
    pub deleted_files: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameEvent {
    pub application_url: String,
    pub instance_name: String,
    pub series: Series,
    pub renamed_episode_files: Vec<RenamedEpisodeFile>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthEvent {
    pub instance_name: String,
    pub level: String,
    pub message: String,
    pub r#type: String,
    pub wiki_url: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthRestoredEvent {
    pub instance_name: String,
    pub level: String,
    pub message: String,
    pub r#type: String,
    pub wiki_url: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUpdateEvent {
    pub application_url: String,
    pub instance_name: String,
    pub message: String,
    pub new_version: String,
    pub previous_version: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManualInteractionRequiredEvent {
    pub application_url: String,
    pub custom_format_info: CustomFormatInfo,
    pub download_client_type: Option<String>,
    pub download_client: Option<String>,
    pub download_id: Option<String>,
    pub download_info: Option<DownloadClientItem>,
    pub download_status_messages: Vec<DownloadStatusMessage>,
    pub download_status: Option<String>,
    pub episodes: Vec<Episode>,
    pub instance_name: String,
    pub release: Release,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "eventType")]
pub enum SonarrEvent {
    Grab(GrabEvent),
    Download(DownloadEvent),
    EpisodeFileDelete(EpisodeFileDeleteEvent),
    SeriesAdd(SeriesAddEvent),
    SeriesDelete(SeriesDeleteEvent),
    Rename(RenameEvent),
    Health(HealthEvent),
    HealthRestored(HealthRestoredEvent),
    ApplicationUpdate(ApplicationUpdateEvent),
    ManualInteractionRequired(ManualInteractionRequiredEvent),
    Test(TestEvent),
}

impl std::fmt::Display for SonarrEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let event_name = match self {
            Self::Grab(_) => "Grab",
            Self::Download(_) => "Download",
            Self::EpisodeFileDelete(_) => "EpisodeFileDelete",
            Self::SeriesAdd(_) => "SeriesAdd",
            Self::SeriesDelete(_) => "SeriesDelete",
            Self::Rename(_) => "Rename",
            Self::Health(_) => "Health",
            Self::HealthRestored(_) => "HealthRestored",
            Self::ApplicationUpdate(_) => "ApplicationUpdate",
            Self::ManualInteractionRequired(_) => "ManualInteractionRequired",
            Self::Test(_) => "Test",
        };
        write!(f, "{}", event_name)
    }
}

pub fn router(
    sender: mpsc::UnboundedSender<SonarrEvent>,
    closer: watch::Sender<bool>,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_webhook))
        .with_state((sender, closer))
}

#[utoipa::path(
    post,
    operation_id = "sonarr_webhook",
    path = "/v4/webhook",
    request_body(content = SonarrEvent, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "Webhook received", body = MessageResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad request", body = MessageResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal server error", body = MessageResponse)
    ),
    tag  = TAG
)]
async fn get_webhook(
    State((state, closer)): State<(mpsc::UnboundedSender<SonarrEvent>, watch::Sender<bool>)>,
    json_str: String,
) -> impl IntoResponse {
    trace!("Event JSON: {}", json_str);
    let data = match serde_json::from_str::<SonarrEvent>(&json_str) {
        Ok(data) => data,
        Err(e) => {
            error!("{}", e.to_string());
            return (
                StatusCode::BAD_REQUEST,
                Json(MessageResponse::new(e.to_string())),
            );
        }
    };
    if let Err(e) = state.send(data) {
        error!("{}", e.to_string());
        if let Err(e) = closer.send(true) {
            error!("Could not send close request: {e}");
        }
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MessageResponse::new(e.to_string())),
        );
    };
    (StatusCode::OK, Json(MessageResponse::ok()))
}
