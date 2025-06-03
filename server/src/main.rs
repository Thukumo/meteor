use std::{collections::{HashMap, VecDeque}, io::Write, net::SocketAddr, sync::Arc};

use axum::{extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, http::StatusCode, response::Response, routing::get_service, Router};
use futures_util::{SinkExt, StreamExt};
use tokio::{sync::{broadcast, Mutex, RwLock}, time::timeout};
use tower_http::services::ServeDir;

const RATE_LIMIT: std::time::Duration = std::time::Duration::ZERO;
const WEBSOCKET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const REMOVE_AFTER: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_HISTORY_SIZE: usize = 100;
const SERVICE_PORT: u16 = 3000;

async fn ws_handler(
    Path(room): Path<String>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(async move |socket| {
        // 削除予定があればキャンセルする
        if let Some(room_state) = state.room_map.read().await.get(&room) {
            let mut status = room_state.status.lock().await;
            match *status {
                RoomState::Active(_) => {},
                RoomState::Inactive(_) => {
                    if let RoomState::Inactive(sender) = std::mem::replace(&mut *status, RoomState::Active(0)) {
                        let _ = sender.send(());
                    }
                }
            }
        }
        let room_state = state.room_map.write().await.entry(room.clone())
            .or_insert_with(|| Room::new(MAX_HISTORY_SIZE)).clone();
        socket_handler(socket, room_state).await;
        let mut room_map = state.room_map.write().await;
        // ルームの接続数が0になったら、ルーム削除のカウントダウンを始める
        let room_map_clone = state.room_map.clone();
        if let Some(room_state) = room_map.get_mut(&room) {
            let mut status = room_state.status.lock().await;
            if let RoomState::Active(connections) = *status {
                if connections == 0 {
                    let (tx, abort) = tokio::sync::oneshot::channel();
                    *status = RoomState::Inactive(tx);
                    tokio::spawn(async move {
                        tokio::select! {
                            _ = tokio::time::sleep(REMOVE_AFTER) => {
                                room_map_clone.write().await.remove(&room);
                            },
                            _ = abort => {}
                        }
                    });
                }
            }
        }
    })
}

async fn socket_handler(socket: WebSocket, room_data: Room) {
    {
        let mut status = room_data.status.lock().await;
        match *status {
            RoomState::Active(ref mut connections) => {
                *connections += 1;
            }
            RoomState::Inactive(_) => {}
        }
    }
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // broadcasterから送信されたメッセージを受信し、WebSocketの送信先に送る
    let mut receiver = room_data.broadcaster.subscribe();
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
                    let _ = room_data.broadcaster.send(text_message.clone());
                    let mut history = room_data.history.write().await;
                    // 履歴がいっぱいなら古いものを削除する
                    if history.len() == history.capacity() {
                        history.pop_front();
                    }
                    history.push_back(text_message.into_text().unwrap().to_string());
                }
                Ok(Message::Close(_)) | Err(_) => { break }
                _ => {} // pingとかは自動で応答してくれるらしい
            }
            tokio::time::sleep(RATE_LIMIT).await;
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
    {
        let mut status = room_data.status.lock().await;
        match *status {
            RoomState::Active(ref mut connections) => {
                *connections -= 1;
            }
            RoomState::Inactive(_) => {}
        }
    }
}

async fn history_handler(
    Path(room): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::Json<VecDeque<String>> {
    axum::Json(
        if let Some(room_data) = state.room_map.read().await.get(&room) {
            room_data.history.read().await.clone()
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
            let map = state.room_map.read().await;
            let mut vec = Vec::with_capacity(map.len());
            for (name, room_data) in map.iter() {
                vec.push(RoomInfo {
                    name: name.clone(),
                    connection: room_data.status.lock().await.get_connections().await
                });
            }
            vec
        }
    )
}

struct AppState {
    // 各ルームの状態を保持するマップ
    room_map: Arc<RwLock<HashMap<String, Room>>>,
}
#[derive(Clone)]
struct Room {
    status: Arc<Mutex<RoomState>>,
    broadcaster: broadcast::Sender<Message>,
    history: Arc<RwLock<VecDeque<String>>>,
}
impl Room {
    fn new(capacity: usize) -> Self {
        Self {
            status: Arc::new(Mutex::new(RoomState::Active(0))),
            broadcaster: broadcast::channel(capacity).0,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
        }
    }
}
enum RoomState {
    // room_state.broadcaster.receiver_count()がダメなので別で値を持っている 購読者数の反映はちょっと遅い?
    Active(usize),
    Inactive(tokio::sync::oneshot::Sender<()>),
}
impl RoomState {
    async fn get_connections(&self) -> usize {
        match self {
            Self::Active(num) => *num,
            Self::Inactive(_) => 0
        }
    }
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(AppState {
        room_map: Arc::new(RwLock::new(HashMap::new())),
    });
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
                        for (name, room_data) in map.iter() {
                            println!("Room: {}, Connections: {}", name, room_data.status.lock().await.get_connections().await);
                        }
                    }
                    _ => {}
                }
            }
            buf.clear();
        }
    }).await.unwrap();
}
