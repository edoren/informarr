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

pub const TAG: &str = "jellyseerr";

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NotificationData {
    pub notification_type: String,
    pub event: String,
    pub subject: String,
    pub message: String,
    pub image: String,
    pub media: Option<Media>,
    pub request: Option<Request>,
    pub issue: Option<Issue>,
    pub comment: Option<Comment>,
    pub extra: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Media {
    pub media_type: String,
    #[serde(rename = "tmdbId")]
    pub tmdb_id: String,
    #[serde(rename = "tvdbId")]
    pub tvdb_id: String,
    pub status: String,
    pub status4k: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Request {
    pub request_id: String,
    #[serde(rename = "requestedBy_email")]
    pub requested_by_email: String,
    #[serde(rename = "requestedBy_username")]
    pub requested_by_username: String,
    #[serde(rename = "requestedBy_avatar")]
    pub requested_by_avatar: String,
    #[serde(rename = "requestedBy_settings_discordId")]
    pub requested_by_settings_discord_id: String,
    #[serde(rename = "requestedBy_settings_telegramChatId")]
    pub requested_by_settings_telegram_chat_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Issue {
    pub issue_id: String,
    pub issue_type: String,
    pub issue_status: String,
    #[serde(rename = "reportedBy_email")]
    pub reported_by_email: String,
    #[serde(rename = "reportedBy_username")]
    pub reported_by_username: String,
    #[serde(rename = "reportedBy_avatar")]
    pub reported_by_avatar: String,
    #[serde(rename = "reportedBy_settings_discordId")]
    pub reported_by_settings_discord_id: String,
    #[serde(rename = "reportedBy_settings_telegramChatId")]
    pub reported_by_settings_telegram_chat_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Comment {
    pub comment_message: String,
    #[serde(rename = "commentedBy_email")]
    pub commented_by_email: String,
    #[serde(rename = "commentedBy_username")]
    pub commented_by_username: String,
    #[serde(rename = "commentedBy_avatar")]
    pub commented_by_avatar: String,
    #[serde(rename = "commentedBy_settings_discordId")]
    pub commented_by_settings_discord_id: String,
    #[serde(rename = "commentedBy_settings_telegramChatId")]
    pub commented_by_settings_telegram_chat_id: String,
}

/// expose the Customer OpenAPI to parent module
pub fn router() -> OpenApiRouter {
    OpenApiRouter::new().routes(routes!(get_webhook))
}

#[utoipa::path(
    post,
    operation_id = "jellyseerr_webhook",
    path = "/v2/webhook",
    request_body(content = NotificationData, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "Webhook received", body = MessageResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad request", body = MessageResponse)
    ),
    tag  = TAG
)]
async fn get_webhook(json: Json<Value>) -> impl IntoResponse {
    let data = match serde_json::from_value::<NotificationData>(json.clone().take()) {
        Ok(data) => data,
        Err(e) => {
            let json_minify = serde_json::to_string(&json.0).unwrap_or_default();
            error!("{e}");
            error!("Error parsing json: {json_minify}");
            return (StatusCode::OK, Json(MessageResponse::new(e.to_string())));
        }
    };
    info!("{data:?}");
    (StatusCode::OK, Json(MessageResponse::ok()))
}
