mod handlers;
mod state;
use std::{net::SocketAddr, sync::Arc};

use axum::{routing::get_service, Router};
use tower_http::services::ServeDir;

use crate::{handlers::{history_handler, ws_handler}, state::AppState};

const SERVICE_PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest("/api", Router::new()
            .nest("/v1", Router::new()
                .nest("/room/{room}", Router::new()
                    .route("/ws", axum::routing::get(ws_handler))
                    .route("/history", axum::routing::get(history_handler))
                )
                // .route("/room_list", axum::routing::get(room_list_handler))
            )
        )
        .fallback_service(get_service(ServeDir::new("static")))
        .with_state(Arc::new(AppState::new()));
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], SERVICE_PORT))).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
