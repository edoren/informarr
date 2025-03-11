use std::net::Ipv4Addr;

use anyhow::Result;
use axum::{
    body::Body,
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod jellyseerr;
mod radarr;
mod sonarr;

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = sonarr::TAG, description = "Sonarr API endpoints"),
        (name = radarr::TAG, description = "Radarr API endpoints"),
        (name = jellyseerr::TAG, description = "Jellyseerr API endpoints")
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

async fn logging_middleware(req: Request<Body>, next: Next) -> Response {
    info!("Received a request to {}", req.uri());
    next.run(req).await
}

#[tokio::main]
async fn main() -> Result<()> {
    edolib::log::setup("informarr").await?;

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1/sonarr", sonarr::router())
        .nest("/api/v1/radarr", radarr::router())
        .nest("/api/v1/jellyseerr", jellyseerr::router())
        .layer(middleware::from_fn(logging_middleware))
        .split_for_parts();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/apidoc/openapi.json", api));

    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 3000)).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
