use std::net::Ipv4Addr;

use anyhow::{Result, anyhow};
use axum::{
    body::Body,
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    net::TcpListener,
    signal,
    sync::{mpsc, watch},
};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod controller;
mod models;
mod schema;
mod webhooks;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = webhooks::sonarr::TAG, description = "Sonarr API endpoints"),
        (name = webhooks::radarr::TAG, description = "Radarr API endpoints"),
        (name = webhooks::jellyseerr::TAG, description = "Jellyseerr API endpoints")
    )
)]
struct ApiDoc;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
struct MessageResponse {
    message: String,
}

impl MessageResponse {
    fn new(message: String) -> Self {
        Self { message }
    }

    fn ok() -> Self {
        Self {
            message: "ok".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct DiscordConfig {
    username: Option<String>,
    avatar_url: Option<String>,
    webhook_url: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TelegramConfig {
    bot_token: String,
    chat_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SonarrConfig {
    url: String,
    api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RadarrConfig {
    url: String,
    api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct JellyseerrConfig {
    url: String,
    api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AppConfig {
    jellyseerr: JellyseerrConfig,
    discord: Option<DiscordConfig>,
    telegram: Option<TelegramConfig>,
    sonarr: Option<Vec<SonarrConfig>>,
    radarr: Option<Vec<RadarrConfig>>,
}

async fn logging_middleware(req: Request<Body>, next: Next) -> Response {
    trace!("Received a call to {}", req.uri());
    next.run(req).await
}

async fn shutdown_signal(close_tx: watch::Sender<bool>, mut close_rx: watch::Receiver<bool>) {
    let ctrl_c = || async move {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = || async move {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = || async move { std::future::pending::<()>().await };

    let send_close = loop {
        tokio::select! {
            _ = ctrl_c() => {
                debug!("Process interrupted");
                break true;
            },
            _ = terminate() => {
                debug!("Process terminated");
                break true;
            },
            result = close_rx.changed() => {
                debug!("Close requested");
                if result.is_ok() && *close_rx.borrow_and_update() {
                    break false;
                }
            }
        }
    };

    if send_close {
        if let Err(e) = close_tx.send(true) {
            error!("Could not send close request: {e}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    edolib::log::setup("informarr").await?;

    let port = std::env::var("INFORMARR_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(7532);

    let content = match fs::read_to_string("config.yml").await {
        Ok(content) => content,
        Err(_) => fs::read_to_string("config.yaml")
            .await
            .map_err(|_e| anyhow!("Could not find config.yml or config.yaml files"))?,
    };

    let app_config = serde_yaml::from_str::<AppConfig>(&content)
        .map_err(|e| anyhow!("Could not parse config: {e}"))?;

    let (sonarr_tx, sonarr_rx) = mpsc::unbounded_channel();
    let (radarr_tx, radarr_rx) = mpsc::unbounded_channel();
    let (jellyseerr_tx, jellyseerr_rx) = mpsc::unbounded_channel();
    let (close_tx, close_rx) = watch::channel(false);

    let worker = tokio::spawn(controller::run(
        app_config,
        sonarr_rx,
        radarr_rx,
        jellyseerr_rx,
        close_tx.clone(),
        close_rx.clone(),
    ));

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(
            "/api/v1/sonarr",
            webhooks::sonarr::router(sonarr_tx, close_tx.clone()),
        )
        .nest(
            "/api/v1/radarr",
            webhooks::radarr::router(radarr_tx, close_tx.clone()),
        )
        .nest(
            "/api/v1/jellyseerr",
            webhooks::jellyseerr::router(jellyseerr_tx, close_tx.clone()),
        )
        .layer(middleware::from_fn(logging_middleware))
        .split_for_parts();

    let router = router.merge(SwaggerUi::new("/").url("/apidoc/openapi.json", api));

    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(close_tx, close_rx))
        .await?;

    if let Err(err) = worker.await? {
        error!("Error on spawned task: {err}");
    }

    Ok(())
}
