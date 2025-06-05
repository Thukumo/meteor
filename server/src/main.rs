mod handlers;
mod state;
use std::{net::SocketAddr, sync::Arc};

use axum::{http::StatusCode, response::Redirect, routing::get_service, Router};
use tower_http::services::ServeDir;

use crate::{handlers::{history_handler, ws_handler}, state::AppState};

const SERVICE_PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", axum::routing::get(|| async {Redirect::permanent("/index.html")}))
        .route_service("/{path}", get_service(ServeDir::new("static")))
        .nest("/api", Router::new()
            .nest("/v1", Router::new()
                .nest("/room/{room}", Router::new()
                    .route("/ws", axum::routing::get(ws_handler))
                    .route("/history", axum::routing::get(history_handler))
                )
                // .route("/room_list", axum::routing::get(room_list_handler))
            )
        )
        .with_state(Arc::new(AppState::new()))
        .fallback_service(axum::routing::get(|| async { StatusCode::NOT_FOUND }));
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], SERVICE_PORT))).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
