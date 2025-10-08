mod handlers;
mod state;
use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    routing::{get, get_service},
};
use tower_http::{
    CompressionLevel,
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};

use crate::{
    handlers::{history_handler, ws_handler},
    state::AppState,
};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest(
            "/api",
            Router::new().nest(
                "/v1",
                Router::new().nest(
                    "/room/{room}",
                    Router::new()
                        .route("/ws", axum::routing::get(ws_handler))
                        .route("/history", axum::routing::get(history_handler)),
                ), // .route("/room_list", axum::routing::get(room_list_handler))
            ),
        )
        .fallback_service(get_service(
            ServeDir::new("static").not_found_service(ServeFile::new("static/index.html")),
        ))
        .with_state(Arc::new(AppState::new()))
        .layer(CompressionLayer::new().quality(CompressionLevel::Best));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
