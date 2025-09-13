use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::MessageResponse;

pub const TAG: &str = "radarr";

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Movie {
    pub folder_path: String,
    pub genres: Option<Vec<String>>,
    pub id: i32,
    pub images: Option<Vec<Image>>,
    pub imdb_id: Option<String>,
    pub original_language: Option<Language>,
    pub overview: Option<String>,
    pub release_date: String,
    pub tags: Vec<String>,
    pub title: String,
    pub tmdb_id: i32,
    pub year: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMovie {
    pub title: String,
    pub images: Option<Vec<Image>>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenamedMovieFile {
    pub previous_relative_path: String,
    pub previous_path: String,
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
pub struct Image {
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
pub struct MovieFile {
    pub date_added: String,
    pub id: i32,
    pub indexer_flags: String,
    pub languages: Vec<Language>,
    pub media_info: MediaInfo,
    pub path: String,
    pub quality: Option<String>,
    pub quality_version: Option<i32>,
    pub relative_path: String,
    pub release_group: Option<String>,
    pub scene_name: Option<String>,
    pub size: i64,
    pub source_path: Option<String>,
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
pub struct GrabEvent {
    pub application_url: String,
    pub custom_format_info: CustomFormatInfo,
    pub download_client_type: String,
    pub download_client: String,
    pub download_id: String,
    pub instance_name: String,
    pub movie: Movie,
    pub release: Release,
    pub remote_movie: Option<RemoteMovie>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub application_url: String,
    pub custom_format_info: CustomFormatInfo,
    pub download_client_type: Option<String>,
    pub download_client: Option<String>,
    pub download_id: Option<String>,
    pub deleted_files: Option<Vec<MovieFile>>,
    pub instance_name: String,
    #[serde(default)]
    pub is_upgrade: bool,
    pub movie_file: MovieFile,
    pub movie: Movie,
    pub release: Release,
    pub remote_movie: Option<RemoteMovie>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadClientItem {
    pub quality: String,
    pub quality_version: i32,
    pub title: String,
    pub indexer: Option<String>,
    pub size: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatusMessage {
    pub title: String,
    pub messages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieAddedEvent {
    pub add_method: String,
    pub application_url: String,
    pub instance_name: String,
    pub movie: Movie,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieFileDeleteEvent {
    pub application_url: String,
    pub delete_reason: String,
    pub instance_name: String,
    pub movie_file: MovieFile,
    pub movie: Movie,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieDeleteEvent {
    pub application_url: String,
    pub instance_name: String,
    pub movie: Movie,
    pub movie_folder_size: i64,
    pub deleted_files: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameEvent {
    pub application_url: String,
    pub instance_name: String,
    pub movie: Movie,
    pub renamed_movie_files: Vec<RenamedMovieFile>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthEvent {
    pub application_url: String,
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
    pub download_status_message: Option<Vec<DownloadStatusMessage>>,
    pub download_status: Option<String>,
    pub instance_name: String,
    pub movie: Option<Movie>,
    pub release: Release,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestEvent {
    pub application_url: String,
    pub instance_name: String,
    pub movie: Movie,
    pub release: Release,
    pub remote_movie: RemoteMovie,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "eventType")]
pub enum RadarrEvent {
    Grab(GrabEvent),
    Download(DownloadEvent),
    MovieAdded(MovieAddedEvent),
    MovieFileDelete(MovieFileDeleteEvent),
    MovieDelete(MovieDeleteEvent),
    Rename(RenameEvent),
    Health(HealthEvent),
    HealthRestored(HealthRestoredEvent),
    ApplicationUpdate(ApplicationUpdateEvent),
    ManualInteractionRequired(ManualInteractionRequiredEvent),
    Test(TestEvent),
}

impl std::fmt::Display for RadarrEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let event_name = match self {
            Self::Grab(_) => "Grab",
            Self::Download(_) => "Download",
            Self::MovieAdded(_) => "MovieAdded",
            Self::MovieFileDelete(_) => "MovieFileDelete",
            Self::MovieDelete(_) => "MovieDelete",
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
    sender: mpsc::UnboundedSender<RadarrEvent>,
    closer: watch::Sender<bool>,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_webhook))
        .with_state((sender, closer))
}

#[utoipa::path(
    post,
    operation_id = "radarr_webhook",
    path = "/v5/webhook",
    request_body(content = RadarrEvent, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "Webhook received", body = MessageResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad request", body = MessageResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal server error", body = MessageResponse)
    ),
    tag  = TAG
)]
async fn get_webhook(
    State((state, closer)): State<(mpsc::UnboundedSender<RadarrEvent>, watch::Sender<bool>)>,
    json_str: String,
) -> impl IntoResponse {
    trace!("Event JSON: {}", json_str);
    let data = match serde_json::from_str::<RadarrEvent>(&json_str) {
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
