use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::MessageResponse;

pub const TAG: &str = "radarr";

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Movie {
    pub folder_path: String,
    pub genres: Option<Vec<String>>,
    pub id: u32,
    pub images: Option<Vec<Image>>,
    pub imdb_id: Option<String>,
    pub original_language: Option<Language>,
    pub overview: Option<String>,
    pub release_date: String,
    pub tags: Vec<String>,
    pub title: String,
    pub tmdb_id: u32,
    pub year: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMovie {
    pub title: String,
    pub images: Option<Vec<Image>>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<u32>,
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
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub custom_format_score: Option<u32>,
    pub custom_formats: Option<Vec<String>>,
    pub indexer: Option<String>,
    pub indexer_flags: Option<Vec<String>>,
    pub languages: Option<Vec<Language>>,
    pub quality: String,
    pub quality_version: u32,
    pub release_group: String,
    pub release_title: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieFile {
    pub date_added: String,
    pub id: u32,
    pub indexer_flags: String,
    pub languages: Vec<Language>,
    pub media_info: MediaInfo,
    pub path: String,
    pub quality: String,
    pub quality_version: u32,
    pub relative_path: String,
    pub release_group: String,
    pub scene_name: String,
    pub size: u64,
    pub source_path: String,
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
pub struct MovieAddedEvent {
    pub add_method: String,
    pub application_url: String,
    pub instance_name: String,
    pub movie: Movie,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieDeleteEvent {
    pub application_url: String,
    pub instance_name: String,
    pub movie: Movie,
    pub movie_folder_size: u64,
    pub deleted_files: bool,
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
    pub download_client: String,
    pub download_client_type: String,
    pub download_id: String,
    pub movie: Movie,
    pub release: Release,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub application_url: String,
    pub instance_name: String,
    pub download_client: String,
    pub download_client_type: String,
    pub download_id: String,
    pub movie: Movie,
    pub movie_file: MovieFile,
    pub release: Release,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "eventType")]
pub enum RadarrEvent {
    MovieAdded(MovieAddedEvent),
    MovieDelete(MovieDeleteEvent),
    Test(TestEvent),
    Health(HealthEvent),
    HealthRestored(HealthEvent),
    Grab(GrabEvent),
    Download(DownloadEvent),
}

/// expose the Customer OpenAPI to parent module
pub fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(get_webhook))
}

#[utoipa::path(
    post,
    operation_id = "radarr_webhook",
    path = "/v5/webhook",
    request_body(content = RadarrEvent, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "Webhook received", body = MessageResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad request", body = MessageResponse)
    ),
    tag  = TAG
)]
async fn get_webhook(json: Json<Value>) -> impl IntoResponse {
    let data = match serde_json::from_value::<RadarrEvent>(json.clone().take()) {
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
