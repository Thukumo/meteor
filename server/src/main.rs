use std::{collections::{HashMap, VecDeque}, sync::Arc};

use axum::{extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, response::Response, routing::get_service, Router};
use futures_util::{SinkExt, StreamExt};
use tokio::{sync::{broadcast, RwLock}, time::timeout};
use tower_http::services::ServeDir;

async fn ws_handler(
    Path(room): Path<String>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    let (sender, _) = broadcast::channel(100);
    let room_data = state.room_map.write().await.entry(room)
        .or_insert_with(|| (sender, Arc::new(RwLock::new(VecDeque::new()))))
        .clone();
    ws.on_upgrade(async |socket| {
        socket_handler(socket, room_data).await;
    })
}

async fn socket_handler(socket: WebSocket, room_data: (broadcast::Sender<Message>, Arc<RwLock<VecDeque<String>>>) ) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // broadcasterから送信されたメッセージを受信し、WebSocketの送信先に送る
    let (broadcaster, history) = room_data;
    let mut receiver = broadcaster.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = receiver.recv().await {
            // 5秒で送信が完了しない場合、切断する
            if timeout(std::time::Duration::from_secs(5), ws_sender.send(message)).await.is_err() {
                break;
            }
        }
    });
    // クライアントからのメッセージ受信の処理
    let mut recv_task = tokio::spawn(async move {
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(text_message @ Message::Text(_)) => {
                    let _ = broadcaster.send(text_message.clone());
                    let mut history = history.write().await;
                    // 履歴は100件まで保持する
                    if history.len() == 100 {
                        history.pop_front();
                    }
                    history.push_back(text_message.into_text().unwrap().to_string());
                }
                Ok(Message::Close(_)) | Err(_) => { break }
                _ => {} // pingとかは自動で応答してくれるらしい
            }
            // tokio::time::sleep(std::time::Duration::from_millis(100)).await; // ここでレート制限的にsleepを入れてもいいかも
        }
    });
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }
}

async fn history_handler(
    Path(room): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::Json<Vec<String>> {
    axum::Json(
        if let Some((_, history)) = state.room_map.read().await.get(&room) {
            history.read().await.iter().cloned().collect()
        } else {
            Vec::new()
        }
    )
}

struct AppState {
    // 各ルームの状態を保持するマップ
    room_map: Arc<RwLock<HashMap<String, (broadcast::Sender<Message>, Arc<RwLock<VecDeque<String>>>)>>>,
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(AppState {
        room_map: Arc::new(RwLock::new(HashMap::new())),
    });
    // 有効なクライアントの接続がないルームを定期的に削除するバックグラウンドタスク
    {
        let app_state = Arc::clone(&app_state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                app_state.room_map.write().await.retain(|_, (sender, _)| {
                    /*
                        pingを送信し、ルームの送信先が空であれば削除
                        senderにpingを送信すると、死んでいるwebsocket接続(及びreceiver)が、少なくとも次のloopまでにdropされるはず
                    */
                    let _ = sender.send(Message::Ping([].as_slice().into()));
                    sender.receiver_count() != 0
                });
            }
        });
    }
    let app = Router::new()
        .route_service("/{path}", get_service(ServeDir::new("static")))
        .nest("/{room}", Router::new()
            .nest("/api", Router::new()
                .nest("/v1", Router::new()
                    .route("/ws", axum::routing::get(ws_handler))
                    .route("/history", axum::routing::get(history_handler))
                )
            )
        )
        .with_state(app_state)
        .fallback_service(axum::routing::get(|| async { "404 Not Found" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
