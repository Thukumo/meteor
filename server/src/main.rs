use std::{collections::HashMap, sync::Arc};

use axum::{extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, response::Response, routing::get_service, Router};
use futures_util::{SinkExt, StreamExt};
use tokio::{sync::{broadcast, Mutex}, time::timeout};
use tower_http::services::ServeDir;

async fn ws_handler(
    Path(room): Path<String>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    let sender = state.room_map.lock().await.entry(room)
        .or_insert_with(|| broadcast::channel(100).0)
        .clone();
    ws.on_upgrade(async |socket| {
        socket_handler(socket, sender).await;
    })
}

async fn socket_handler(socket: WebSocket, broadcaster: broadcast::Sender<Message>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // broadcasterから送信されたメッセージを受信し、WebSocketの送信先に送る
    let broadcaster_clone = broadcaster.clone();
    tokio::spawn(async move {
        let mut receiver = broadcaster_clone.subscribe();
        while let Ok(message) = receiver.recv().await {
            // 5秒で送信が完了しない場合、切断する
            if timeout(std::time::Duration::from_secs(5), ws_sender.send(message)).await.is_err() {
                break;
            }
        }
    });
    // クライアントからのメッセージ受信の処理
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(text_message @ Message::Text(_)) => { if broadcaster.send(text_message).is_err() { break } }
            Ok(Message::Close(_)) | Err(_) => { break }
            _ => {} // pingとかは自動で応答してくれるらしい
        }
    }
}

struct AppState {
    // ルーム名をキーとした、broadcastのSenderを保持するマップ
    room_map: Arc<Mutex<HashMap<String, broadcast::Sender<Message>>>>,
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(AppState {
        room_map: Arc::new(Mutex::new(HashMap::new())),
    });
    // 有効なクライアントの接続がないルームを定期的に削除するバックグラウンドタスク
    {
        let app_state = Arc::clone(&app_state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                let mut remove_rooms = Vec::new();
                let mut map_lock = app_state.room_map.lock().await;
                for (room, sender) in map_lock.iter() {
                    /*
                        pingを送信し、ルームの送信先が空であれば削除
                        senderにpingを送信すると、if ws_sender.send(message).await.is_err() {break;} が発火して、
                        有効でないwebsocket接続(及びreceiver)が、少なくとも次のloopまでにdropされるはず
                    */
                    let _ = sender.send(Message::Ping(Vec::new().into()));
                    if sender.receiver_count() == 0 {
                        println!("Removing room: {}", room);
                        remove_rooms.push(room.clone());
                    }
                }
                for room in remove_rooms {
                    map_lock.remove(&room);
                }
            }
        });
    }
    let app = Router::new()
        .route_service("/{path}", get_service(ServeDir::new("static")))
        .nest("/{room}", Router::new()
            .nest("/api", Router::new()
                .nest("/v1", Router::new()
                    .route("/ws", axum::routing::get(ws_handler))
                    // .route("/comment", axum::routing::post(post_comment))
                )
            )
        )
        .with_state(app_state)
        .fallback_service(axum::routing::get(|| async { "404 Not Found" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
