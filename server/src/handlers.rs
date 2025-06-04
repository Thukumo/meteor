use std::{collections::VecDeque, sync::Arc};

use axum::{extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, response::Response};
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;

use crate::state::{AppState, Room};

const WEBSOCKET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub async fn ws_handler(
    Path(room_name): Path<String>,
    State(state): State<Arc<crate::state::AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(async move |socket| {
        let room = state.get_or_create_room(&room_name).await;
        // ルームがアクティブでない場合は、アクティブにする
        room.activate_or_nop().await;
        let room_clone = room.clone();
        // WebSocketハンドラーの実処理
        socket_handler(socket, room).await;
        // ルームの接続数が0になったら、ルーム削除のカウントダウンを始める
        if room_clone.get_connections().await == 0 {
            state.remove_room(&room_name).await;
        }
    })
}

async fn socket_handler(socket: WebSocket, room: Room) {
    room.increment_connections().await;
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // broadcasterから送信されたメッセージを受信し、WebSocketの送信先に送る
    let mut receiver = room.broadcaster.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = receiver.recv().await {
            // 設定した時間内にメッセージを送信できなかった場合、終了する
            if timeout(WEBSOCKET_TIMEOUT, ws_sender.send(message)).await.is_err() {
                break;
            }
        }
    });
    let room_clone = room.clone();
    // クライアントからのメッセージ受信の処理
    let mut recv_task = tokio::spawn(async move {
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(text_message @ Message::Text(_)) => {
                    let _ = room.broadcaster.send(text_message.clone());
                    room.add_history(text_message.into_text().unwrap().to_string()).await;
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
    room_clone.decrement_connections().await;
}

pub async fn history_handler(
    Path(room_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::Json<VecDeque<String>> {
    axum::Json(
        if let Some(room) = state.get_room(&room_name).await {
            room.get_history().await
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
                let connections = room_data.get_connections().await;
                vec.push(RoomInfo {
                    name: name.clone(),
                    connection: connections,
                });
            }
            vec
        }
    )
}
