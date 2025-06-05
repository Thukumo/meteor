use std::{collections::{HashMap, VecDeque}, sync::Arc};

use axum::extract::ws::Message;
use tokio::sync::{broadcast, RwLock};

const MAX_HISTORY_SIZE: usize = 100;
const REMOVE_AFTER: std::time::Duration = std::time::Duration::from_secs(20);

pub struct AppState {
    // 各ルームの状態を保持するマップ
    pub room_map: Arc<RwLock<HashMap<String, Arc<RwLock<Room>>>>>,
}
impl AppState {
    pub fn new() -> Self {
        Self {
            room_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    // ルームを取得、存在しない場合作成
    pub async fn get_or_create_room(&self, room_name: &str) -> Arc<RwLock<Room>> {
        self.room_map.write().await.entry(room_name.to_string()).or_insert_with(|| Arc::new(RwLock::new(Room::new()))).clone()
    }
    // ルームを取得、Optionで返す
    pub async fn get_room(&self, room_name: &str) -> Option<Arc<RwLock<Room>>> {
        let room_map = self.room_map.read().await;
        room_map.get(room_name).cloned()
    }
    pub async fn connect(&self, room_name: &str) -> Arc<RwLock<Room>> {
        let room = self.get_or_create_room(room_name).await;
        {
            let mut room_write = room.write().await;
            room_write.increment_connections().await;
            if let RoomState::Inactive(sender) = std::mem::replace(&mut *room_write.status.write().await, RoomState::Active) {
                let _ = sender.send(());
            }
        }
        room.clone()
    }
    // これが呼び出されるときは、connectを呼び出した後であるのでroomは確実に存在する(としておく)
    pub async fn disconnect(&self, room_name: &str) {
        if let Some(room) = self.get_room(room_name).await {
            room.write().await.decrement_connections().await;
        }
    }
    // ルームの削除タスクをスポーンする ルームが使われているなら何もしない
    pub async fn check(&self, room_name: &str) -> Option<()> {
        let room = self.get_room(room_name).await?;
        let room = room.write().await;
        let mut room_status = room.status.write().await;
        if let RoomState::Inactive(_) = *room_status {
            return None;
        } else if room.connection_count != 0 {
            return None;
        }
        let (tx, abort) = tokio::sync::oneshot::channel();
        *room_status = RoomState::Inactive(tx);
        let room_name = room_name.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(REMOVE_AFTER) => {
                    state.room_map.write().await.remove(&room_name);
                },
                _ = abort => {}
            }
        });
        Some(())
    }
}

#[derive(Clone)]
pub struct Room {
    // Room.broadcaster.receiver_count()がダメなので別で値を持っている 購読者数の反映はちょっと遅い?
    connection_count: usize,
    // tokio::sync::oneshot::Sender<()>がCloneを実装していないので、statusはArcにしておく必要がある
    status: Arc<RwLock<RoomState>>,
    pub broadcaster: broadcast::Sender<Message>,
    history: VecDeque<String>,
}
impl Room {
    fn new() -> Self {
        Self {
            connection_count: 0,
            status: Arc::new(RwLock::new(RoomState::Active)),
            broadcaster: broadcast::channel(MAX_HISTORY_SIZE).0,
            history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
        }
    }
    // コネクション数をインクリメント
    async fn increment_connections(&mut self) {
        self.connection_count += 1;
        let mut status = self.status.write().await;
        match *status {
            RoomState::Active => {},
            RoomState::Inactive(_) => {
                if let RoomState::Inactive(sender) = std::mem::replace(&mut *status, RoomState::Active) {
                    let _ = sender.send(());
                }
            }
        }
    }
    // コネクション数をデクリメント
    async fn decrement_connections(&mut self) {
        self.connection_count -= 1;
        
    }
    // 現在のコネクション数を取得
    pub async fn get_connections(&self) -> usize {
        self.connection_count
    }
    async fn activate_or_nop(&mut self) {
        // この下の処理の間にroomがHashMapからremoveされても、参照カウンタ的にこのメソッドを実行し終わった段階でDropされるはず
        let mut status = self.status.write().await;
        match *status {
            RoomState::Active => {},
            RoomState::Inactive(_) => {
                if let RoomState::Inactive(sender) = std::mem::replace(&mut *status, RoomState::Active) {
                    let _ = sender.send(());
                }
            }
        }
    }
    // メッセージを履歴に追加
    pub async fn add_history(&mut self, message: String) {
        // 履歴がいっぱいなら古いものを削除する
        if self.history.len() == MAX_HISTORY_SIZE {
            self.history.pop_front();
        }
        self.history.push_back(message);
    }
    // 履歴を取得
    pub async fn get_history(&self) -> VecDeque<String> {
        self.history.clone()
    }
}
enum RoomState {
    Active,
    Inactive(tokio::sync::oneshot::Sender<()>),
}
