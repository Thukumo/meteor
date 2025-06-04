mod handlers;
mod state;
use std::{io::Write, net::SocketAddr, sync::Arc};

use axum::{http::StatusCode, routing::get_service, Router};
use tower_http::services::ServeDir;

use crate::handlers::{ws_handler, history_handler};

const SERVICE_PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    let app_state = Arc::new(state::AppState::new());
    let app_state_clone = app_state.clone();
    let app = Router::new()
        .route_service("/", get_service(ServeDir::new("static")))
        .nest("/api", Router::new()
            .nest("/v1", Router::new()
                .nest("/room/{room}", Router::new()
                    .route("/ws", axum::routing::get(ws_handler))
                    .route("/history", axum::routing::get(history_handler))
                )
                // .route("/room_list", axum::routing::get(room_list_handler))
            )
        )
        .with_state(app_state)
        .fallback_service(axum::routing::get(|| async { StatusCode::NOT_FOUND }));
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], SERVICE_PORT))).await.unwrap();
    axum::serve(listener, app).with_graceful_shutdown(async move {
        let mut buf = String::new();
        loop {
            print!("> ");
            let _ = std::io::stdout().flush();
            if std::io::stdin().read_line(&mut buf).is_err() {
                continue;
            }
            let input = buf.trim().split_ascii_whitespace().collect::<Vec<_>>();
            if let Some(command) = input.get(0) {
                match *command {
                    "exit" | "quit" | "stop" => {
                        println!("Shutting down server...");
                        break;
                    }
                    "room" | "rooms" => {
                        let map = app_state_clone.room_map.read().await;
                        println!("{} active rooms:", map.len());
                        for (name, room) in map.iter() {
                            println!("Room: {}, Connections: {}", name, room.get_connections().await);
                        }
  
                    }
                    _ => {}
                }
            }
            buf.clear();
        }
    }).await.unwrap();
}
