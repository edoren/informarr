use std::collections::HashMap;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use log::{error, trace};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use serde_json::Value;
use tokio::sync::{mpsc, watch};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::MessageResponse;

pub const TAG: &str = "jellyseerr";

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    TV,
    Movie,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaStatus {
    Available,
    Blacklisted,
    Deleted,
    PartiallyAvailable,
    Pending,
    Processing,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Media {
    pub media_type: String,
    #[serde(
        rename = "tmdbId",
        deserialize_with = "deserialize_option_number_from_string"
    )]
    pub tmdb_id: Option<i32>,
    #[serde(
        rename = "tvdbId",
        deserialize_with = "deserialize_option_number_from_string"
    )]
    pub tvdb_id: Option<i32>,
    pub status: MediaStatus,
    pub status4k: MediaStatus,
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

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueType {
    Video,
    Audio,
    Subtitles,
    Other,
}

// impl IssueType {
//     pub fn name(&self) -> &str {
//         match self {
//             Self::Video => "Video",
//             Self::Audio => "Audio",
//             Self::Subtitles => "Subtitles",
//             Self::Other => "Other",
//         }
//     }
// }

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueStatus {
    Open,
    Resolved,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Issue {
    pub issue_id: String,
    pub issue_type: IssueType,
    pub issue_status: IssueStatus,
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExtraData {
    pub name: String,
    pub value: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    None,
    MediaPending,
    MediaApproved,
    MediaAvailable,
    MediaFailed,
    TestNotification,
    MediaDeclined,
    MediaAutoApproved,
    IssueCreated,
    IssueComment,
    IssueResolved,
    IssueReopened,
    MediaAutoRequested,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let event_name = match self {
            Self::None => "None",
            Self::MediaPending => "MediaPending",
            Self::MediaApproved => "MediaApproved",
            Self::MediaAvailable => "MediaAvailable",
            Self::MediaFailed => "MediaFailed",
            Self::TestNotification => "TestNotification",
            Self::MediaDeclined => "MediaDeclined",
            Self::MediaAutoApproved => "MediaAutoApproved",
            Self::IssueCreated => "IssueCreated",
            Self::IssueComment => "IssueComment",
            Self::IssueResolved => "IssueResolved",
            Self::IssueReopened => "IssueReopened",
            Self::MediaAutoRequested => "MediaAutoRequested",
        };
        write!(f, "{}", event_name)
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JellyseerrEvent {
    pub notification_type: NotificationType,
    pub event: String,
    pub subject: String,
    pub message: String,
    pub image: String,
    pub media: Option<Media>,
    pub request: Option<Request>,
    pub issue: Option<Issue>,
    pub comment: Option<Comment>,
    pub extra: Vec<ExtraData>,
}

pub fn router(
    sender: mpsc::UnboundedSender<JellyseerrEvent>,
    closer: watch::Sender<bool>,
) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(get_webhook))
        .with_state((sender, closer))
}

#[utoipa::path(
    post,
    operation_id = "jellyseerr_webhook",
    path = "/v2/webhook",
    request_body(content = JellyseerrEvent, content_type = "application/json"),
    responses(
        (status = StatusCode::OK, description = "Webhook received", body = MessageResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad request", body = MessageResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal server error", body = MessageResponse)
    ),
    tag  = TAG
)]
async fn get_webhook(
    State((state, closer)): State<(mpsc::UnboundedSender<JellyseerrEvent>, watch::Sender<bool>)>,
    mut json: Json<Value>,
) -> impl IntoResponse {
    let json_value = json.take();
    trace!(
        "Event JSON: {}",
        serde_json::to_string(&json_value).unwrap_or("Error Parsing".to_string())
    );
    let data = match serde_json::from_value::<JellyseerrEvent>(json_value) {
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
