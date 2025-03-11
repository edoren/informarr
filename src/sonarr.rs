use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use log::{error, info};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::MessageResponse;

pub const TAG: &str = "sonarr";

use serde::{Deserialize, Serialize};

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
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub genres: Option<Vec<String>>,
    pub id: u32,
    pub images: Option<Vec<SeriesImage>>,
    pub imdb_id: Option<String>,
    pub original_language: Option<Language>,
    pub path: String,
    pub tags: Option<Vec<String>>,
    pub title: String,
    pub title_slug: Option<String>,
    pub tmdb_id: u32,
    pub tv_maze_id: u32,
    pub tvdb_id: u32,
    pub r#type: String,
    pub year: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub episode_number: u32,
    pub id: u32,
    pub season_number: u32,
    pub series_id: u32,
    pub title: String,
    pub tvdb_id: u32,
    pub air_date: Option<String>,
    pub air_date_utc: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormat {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormatInfo {
    pub custom_format_score: u32,
    pub custom_formats: Vec<CustomFormat>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub custom_format_score: Option<u32>,
    pub custom_formats: Vec<String>,
    pub indexer: String,
    pub languages: Vec<Language>,
    pub quality: String,
    pub quality_version: u32,
    pub release_title: String,
    pub size: u64,
    pub release_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub audio_channels: f32,
    pub audio_codec: String,
    pub audio_languages: Vec<String>,
    pub height: u32,
    pub subtitles: Vec<String>,
    pub video_codec: String,
    pub video_dynamic_range: String,
    pub video_dynamic_range_type: String,
    pub width: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFile {
    pub date_added: String,
    pub id: u32,
    pub languages: Vec<Language>,
    pub media_info: MediaInfo,
    pub path: String,
    pub quality: Option<String>,
    pub quality_version: u32,
    pub relative_path: String,
    pub release_group: Option<String>,
    pub size: u64,
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
pub struct TestEvent {
    pub application_url: String,
    pub instance_name: String,
    pub series: Series,
    pub episodes: Vec<Episode>,
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
    pub instance_name: String,
    pub series: Series,
    pub episodes: Vec<Episode>,
    pub episode_files: Vec<EpisodeFile>,
    pub destination_path: String,
    pub source_path: Option<String>,
    pub file_count: u32,
    pub release: Release,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "eventType")]
pub enum SonarrEvent {
    /// SeriesAdd event
    SeriesAdd(SeriesAddEvent),
    /// SeriesDelete event
    SeriesDelete(SeriesDeleteEvent),
    /// Test event
    Test(TestEvent),
    /// Health event
    Health(HealthEvent),
    /// HealthRestored event
    HealthRestored(HealthEvent),
    /// Grab event
    Grab(GrabEvent),
    /// Download event
    Download(DownloadEvent),
}

/// expose the Customer OpenAPI to parent module
pub fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(get_webhook))
}

#[utoipa::path(
    post,
    operation_id = "sonarr_webhook",
    path = "/v4/webhook",
    request_body(content = SonarrEvent, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "Webhook received", body = MessageResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad request", body = MessageResponse)
    ),
    tag  = TAG
)]
async fn get_webhook(json: Json<Value>) -> impl IntoResponse {
    let data = match serde_json::from_value::<SonarrEvent>(json.clone().take()) {
        Ok(data) => data,
        Err(e) => {
            let json_minify = serde_json::to_string(&json.0).unwrap_or_default();
            error!("{e}");
            error!("Error parsing json: {json_minify}");
            return (StatusCode::OK, Json(MessageResponse::new(e.to_string())));
        }
    };
    info!("Parsed correctly: {:?}", data);
    (StatusCode::OK, Json(MessageResponse::ok()))
}
