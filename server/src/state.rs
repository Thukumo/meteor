use std::{collections::{HashMap, VecDeque}, sync::Arc};

use axum::extract::ws::Message;
use tokio::sync::{broadcast, Mutex, RwLock};

const MAX_HISTORY_SIZE: usize = 100;
const REMOVE_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

pub struct AppState {
    // 各ルームの状態を保持するマップ
    pub room_map: Arc<RwLock<HashMap<String, Room>>>,
}
impl AppState {
    pub fn new() -> Self {
        Self {
            room_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    // ルームを取得、存在しない場合作成
    pub async fn get_or_create_room(&self, room_name: &str) -> Room {
        self.room_map.write().await.entry(room_name.to_string()).or_insert_with(Room::new).clone()
    }
    // ルームを取得、Optionで返す
    pub async fn get_room(&self, room_name: &str) -> Option<Room> {
        let room_map = self.room_map.read().await;
        room_map.get(room_name).cloned()
    }
    // ルームの削除タスクをスポーンする
    pub async fn remove_room(&self, room_name: &str) -> Option<()> {
        let room = self.get_room(room_name).await?;
        let mut room = room.status.lock().await;
        assert_eq!(room.get_connections().await, 0, "Cannot remove a room with active connections");
        let (tx, abort) = tokio::sync::oneshot::channel();
        *room = RoomState::Inactive(tx);
        let room_name = room_name.to_string();
        let room_map = self.room_map.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(REMOVE_AFTER) => {
                    let mut room_map = room_map.write().await;
                    room_map.remove(&room_name);
                },
                _ = abort => {}
            }
        });
        Some(())
    }
}

#[derive(Clone)]
pub struct Room {
    status: Arc<Mutex<RoomState>>,
    pub broadcaster: broadcast::Sender<Message>,
    history: Arc<RwLock<VecDeque<String>>>,
}
impl Room {
    fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(RoomState::Active(0))),
            broadcaster: broadcast::channel(MAX_HISTORY_SIZE).0,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
        }
    }
    // コネクション数をインクリメント
    pub async fn increment_connections(&self) {
        let mut status = self.status.lock().await;
        match *status {
            RoomState::Active(ref mut connections) => {
                *connections += 1;
            }
            RoomState::Inactive(_) => {}
        }
    }
    // コネクション数をデクリメント
    pub async fn decrement_connections(&self) {
        let mut status = self.status.lock().await;
        match *status {
            RoomState::Active(ref mut connections) => {
                *connections -= 1;
            }
            RoomState::Inactive(_) => {}
        }
    }
    // ルームが削除処理待ちである場合、処理を中断しアクティブにする
    pub async fn activate_or_nop(&self) {
        let mut status = self.status.lock().await;
        match *status {
            RoomState::Active(_) => {},
            RoomState::Inactive(_) => {
                if let RoomState::Inactive(sender) = std::mem::replace(&mut *status, RoomState::Active(0)) {
                    let _ = sender.send(());
                }
            }
        }
    }
    // 現在のコネクション数を取得
    pub async fn get_connections(&self) -> usize {
        let status = self.status.lock().await;
        status.get_connections().await
    }
    // メッセージを履歴に追加
    pub async fn add_history(&self, message: String) {
        let mut history = self.history.write().await;
        // 履歴がいっぱいなら古いものを削除する
        if history.len() == MAX_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(message);
    }
    // 履歴を取得
    pub async fn get_history(&self) -> VecDeque<String> {
        let history = self.history.read().await;
        history.clone()
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
