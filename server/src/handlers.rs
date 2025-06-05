use std::{collections::VecDeque, sync::Arc};

use axum::{extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, response::Response};
use futures_util::{SinkExt, StreamExt};
use tokio::{sync::RwLock, time::timeout};

use crate::state::{AppState, Room};

const WEBSOCKET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub async fn ws_handler(
    Path(room_name): Path<String>,
    State(state): State<Arc<crate::state::AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(async move |socket| {
        let room = state.connect(&room_name).await;
        // ルームがアクティブでない場合は、アクティブにする
        // WebSocketハンドラーの実処理
        socket_handler(socket, room).await;
        // ルームの接続数を確認し、接続がなければ削除タスクをスポーン
        if state.check(&room_name).await.is_none() {
            // tracing::info!("No removal task was scheduled for room: {}", room_name);
        }
    })
}

async fn socket_handler(socket: WebSocket,room: Arc<RwLock<Room>>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // broadcasterから送信されたメッセージを受信し、WebSocketの送信先に送る
    let broadcaster = room.read().await.broadcaster.clone();
    let mut receiver = broadcaster.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = receiver.recv().await {
            // 設定した時間内にメッセージを送信できなかった場合、終了する
            if timeout(WEBSOCKET_TIMEOUT, ws_sender.send(message)).await.is_err() {
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
                    room.write().await.add_history(text_message.into_text().unwrap().to_string()).await;
                }
                Ok(Message::Close(_)) | Err(_) => { break }
                _ => {} // pingとかは自動で応答してくれるらしい
            }
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

pub async fn history_handler(
    Path(room_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::Json<VecDeque<String>> {
    axum::Json(
        if let Some(room) = state.get_room(&room_name).await {
            room.read().await.get_history().await
        } else {
            VecDeque::new()
        }
    )
}
#[derive(serde::Serialize, serde::Deserialize)]
struct RoomInfo {
    name: String,
    connection: usize,
}
#[allow(dead_code)]
async fn room_list_handler(
    State(state): State<Arc<AppState>>,
) -> axum::Json<Vec<RoomInfo>> {
    axum::Json(
        {
            let mut vec = Vec::new();
            let room_map = state.room_map.read().await;
            for (name, room_data) in room_map.iter() {
                let connections = room_data.read().await.get_connections().await;
                vec.push(RoomInfo {
                    name: name.clone(),
                    connection: connections,
                });
            }
            vec
        }
    )
}
