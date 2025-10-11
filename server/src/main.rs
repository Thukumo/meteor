mod handlers;
mod state;
use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    routing::{get, get_service},
};
use log::info;
use tokio::signal::unix;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::prelude::*;

use crate::{
    handlers::{history_handler, ws_handler},
    state::AppState,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
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
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    info!("ポート{}でサーブを開始します。", port);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            unix::signal(unix::SignalKind::terminate())
                .unwrap()
                .recv()
                .await
                .unwrap()
        })
        .await
        .unwrap();
}
