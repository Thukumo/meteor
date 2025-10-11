use std::{collections::VecDeque, sync::Arc};

use axum::{
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use log::warn;
use tokio::{sync::oneshot, time::timeout};

use crate::state::{AppState, Room};

const WEBSOCKET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[tracing::instrument]
pub async fn ws_handler(
    Path(room_name): Path<String>,
    State(state): State<Arc<crate::state::AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(async move |socket| {
        let room = state.get_or_create_room(&room_name).await;
        room.increment_connection().await;
        socket_handler(socket, room.clone()).await;
        room.decrement_connection_and_check().await;
    })
}

#[tracing::instrument]
async fn socket_handler(socket: WebSocket, room: Room) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let ((recv_stop_tx, recv_stop_rx), (send_stop_tx, send_stop_rx)) =
        (oneshot::channel(), oneshot::channel());
    // broadcasterから送信されたメッセージを受信し、WebSocketの送信先に送る
    let (broadcaster, mut receiver) = room.get_tx_rx();
    let mut send_task = tokio::spawn(async move {
        let mut stop = recv_stop_rx;
        while let Ok(message) = tokio::select! {
            message = receiver.recv() => message,
            _ = &mut stop => Err(tokio::sync::broadcast::error::RecvError::Closed),
        } {
            // 設定した時間内にメッセージを送信できなかった場合、終了する
            if timeout(WEBSOCKET_TIMEOUT, ws_sender.send(message))
                .await
                .is_err()
            {
                warn!("WebSocketのメッセージ送信に失敗しました。");
                break;
            }
        }
    });
    // クライアントからのメッセージ受信の処理
    let mut recv_task = tokio::spawn(async move {
        let mut stop = send_stop_rx;
        while let Some(message) = tokio::select! {
            message = ws_receiver.next() => message,
            _ = &mut stop => None,
        } {
            match message {
                Ok(text_message @ Message::Text(_)) => {
                    let _ = broadcaster.send(text_message.clone());
                    room.add_history(text_message.into_text().unwrap().to_string())
                        .await;
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {} // pingとかは自動で応答してくれるらしい
            }
        }
    });
    tokio::select! {
        _ = &mut send_task => {
            let _ = recv_stop_tx.send(());
        }
        _ = &mut recv_task => {
            let _ = send_stop_tx.send(());
        }
    }
}

pub async fn history_handler(
    Path(room_name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::Json<VecDeque<String>> {
    axum::Json(state.get_room(&room_name).await.unwrap_or_default())
}
#[derive(serde::Serialize)]
pub struct RoomInfo {
    pub name: String,
    pub connection: usize,
}
#[allow(dead_code)]
pub async fn room_list_handler(State(state): State<Arc<AppState>>) -> axum::Json<Vec<RoomInfo>> {
    axum::Json(state.get_room_list().await)
}
