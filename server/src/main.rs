use std::{collections::{HashMap, VecDeque}, io::Write, net::SocketAddr, sync::Arc};

use axum::{extract::{ws::{Message, WebSocket}, Path, State, WebSocketUpgrade}, response::Response, routing::get_service, Router};
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
    let room_data = state.room_map.write().await.entry(room.clone())
        .or_insert_with(|| RoomState::new(MAX_HISTORY_SIZE))
        .clone();
    ws.on_upgrade(async move |socket| {
        socket_handler(socket, room_data).await;
        let mut room_map = state.room_map.write().await;
        // ルームの接続数が0になったら、ルーム削除のカウントダウンを始める
        if let Some(room_state) = room_map.get_mut(&room) {
            let check = room_state.destroyer.lock().await.is_none();
            if *room_state.connection.read().await == 0 && check {
                let mut map = state.room_map.read().await.clone();
                *room_state.destroyer.lock().await = Some(tokio::spawn(async move {
                    tokio::time::sleep(REMOVE_AFTER).await;
                    map.remove(&room);
                }));
            }
        }
    })
}

async fn socket_handler(socket: WebSocket, room_data: RoomState) {
    // 削除予定があればキャンセルする
    if let Some(destroyer) = room_data.destroyer.lock().await.take() {
        destroyer.abort();
    }
    *room_data.connection.write().await += 1;
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
    *room_data.connection.write().await -= 1;
}

async fn history_handler(
    Path(room): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::Json<Vec<String>> {
    axum::Json(
        if let Some(room_data) = state.room_map.read().await.get(&room) {
            room_data.history.read().await.iter().cloned().collect()
        } else {
            Vec::new()
        }
    )
}
#[derive(serde::Serialize, serde::Deserialize)]
struct RoomInfo {
    name: String,
    connection: usize,
}
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
                    connection: *room_data.connection.read().await,
                });
            }
            vec
        }
    )
}

struct AppState {
    // 各ルームの状態を保持するマップ
    room_map: Arc<RwLock<HashMap<String, RoomState>>>,
}
#[derive(Clone)]
struct RoomState {
    connection: Arc<RwLock<usize>>,
    broadcaster: broadcast::Sender<Message>,
    history: Arc<RwLock<VecDeque<String>>>,
    destroyer: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}
impl RoomState {
    fn new(capacity: usize) -> Self {
        Self {
            connection: Arc::new(RwLock::new(0)),
            broadcaster: broadcast::channel(capacity).0,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            destroyer: Arc::new(Mutex::new(None)),
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
        .route_service("/{path}", get_service(ServeDir::new("static")))
        .nest("/api", Router::new()
            .nest("/v1", Router::new()
                .nest("/room/{room}", Router::new()
                    .route("/ws", axum::routing::get(ws_handler))
                    .route("/history", axum::routing::get(history_handler))
                )
                .route("/room_list", axum::routing::get(room_list_handler))
            )
        )
        .with_state(app_state)
        .fallback_service(axum::routing::get(|| async { "404 Not Found" }));
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
                            println!("Room: {}, Connections: {}", name, *room_data.connection.read().await);
                        }
                    }
                    _ => {}
                }
            }
            buf.clear();
        }
    }).await.unwrap();
}
