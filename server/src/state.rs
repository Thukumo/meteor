#![allow(dead_code)]
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Weak},
};

use axum::extract::ws::Message;
use log::{debug, info, warn};
use tokio::{
    sync::{RwLock, broadcast},
    task::JoinHandle,
};

use crate::handlers::RoomInfo;

const MAX_HISTORY_SIZE: usize = 100;
const REMOVE_AFTER: std::time::Duration = std::time::Duration::from_secs(60);

type AppStateInner = RwLock<HashMap<String, Room>>;
#[derive(Clone, Default, Debug)]
pub struct AppState(Arc<AppStateInner>);

impl AppState {
    pub fn new() -> Self {
        AppState::default()
    }
    pub fn new_room(&self, name: &str) -> Room {
        let (broadcaster, _) = broadcast::channel(MAX_HISTORY_SIZE);
        info!("ルーム \"{}\"を作成しました。", name);
        Room {
            name: name.to_string(),
            status: Arc::new(RwLock::new(RoomStatus::Active(0))),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            broadcaster,
            parent: Arc::downgrade(&self.0),
        }
    }
    pub async fn get_or_create_room(&self, name: &str) -> Room {
        let room = self
            .0
            .write()
            .await
            .entry(name.to_string())
            .or_insert_with(|| self.new_room(name))
            .clone();
        let room_clone = room.clone();
        let mut status = room_clone.status.write().await;
        if let RoomStatus::Inactive(handle) = &*status {
            handle.abort();
            *status = RoomStatus::Active(0);
            info!("ルーム \"{}\"は再アクティブ化されました。", name);
        }
        room
    }
    pub async fn get_room_list(&self) -> Vec<RoomInfo> {
        let vec: Vec<_> = {
            let map = self.0.read().await;
            map.iter()
                .map(|(name, room)| (name.clone(), room.status.clone()))
                .collect()
        };
        let mut res = Vec::with_capacity(vec.len());
        for (name, status) in vec.into_iter() {
            res.push(RoomInfo {
                name,
                connection: match *status.read().await {
                    RoomStatus::Active(count) => count,
                    RoomStatus::Inactive(_) => 0,
                },
            });
        }
        res
    }
    pub async fn get_room(&self, name: &str) -> Option<VecDeque<String>> {
        let room = self.0.read().await.get(name)?.clone();
        Some(room.get_history().await)
    }
}

#[derive(Clone, Debug)]
pub struct Room {
    name: String,
    status: Arc<RwLock<RoomStatus>>,
    history: Arc<RwLock<VecDeque<String>>>,
    broadcaster: broadcast::Sender<Message>,
    parent: Weak<AppStateInner>,
}
impl Room {
    pub async fn add_history(&self, message: String) {
        let mut history = self.history.write().await;
        if history.len() == MAX_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(message);
    }
    pub async fn get_history(&self) -> VecDeque<String> {
        self.history.read().await.clone()
    }
    pub fn get_tx_rx(&self) -> (broadcast::Sender<Message>, broadcast::Receiver<Message>) {
        (self.broadcaster.clone(), self.broadcaster.subscribe())
    }
    pub async fn is_active(&self) -> bool {
        matches!(*self.status.read().await, RoomStatus::Active(_))
    }
    pub async fn increment_connection(&self) {
        let mut status = self.status.write().await;
        if let RoomStatus::Active(count) = &*status {
            // 書いたはいいけどこれ要ります?
            if *count == usize::MAX {
                warn!(
                    "コネクション数のカウントがusize::MAX({})であるルーム\"{}\"のカウントをインクリメントしようとしました。",
                    usize::MAX,
                    self.name
                );
            } else {
                debug!(
                    "ルーム \"{}\"のコネクション数がインクリメントされました。({}→{})",
                    self.name,
                    count,
                    count + 1
                );
                *status = RoomStatus::Active(count + 1);
            }
        }
    }
    pub async fn decrement_connection_and_check(&self) {
        use RoomStatus::*;
        let mut status = self.status.write().await;
        match &*status {
            Active(count) => {
                if *count == 0 {
                    warn!(
                        "コネクション数のカウントが0であるルーム \"{}\"のカウントをデクリメントしようとしました。",
                        self.name
                    );
                } else if *count == 1 {
                    let room_name = self.name.clone();
                    let parent = self.parent.clone();
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(REMOVE_AFTER).await;
                        if let Some(parent) = parent.upgrade() {
                            parent.write().await.remove(&room_name);
                            info!("ルーム \"{}\"が削除されました。", room_name);
                        }
                    });
                    info!(
                        "ルーム \"{}\"の接続数がデクリメントされ、削除待機状態に移行しました。({}→{})",
                        self.name,
                        count,
                        count - 1
                    );
                    *status = Inactive(handle);
                } else {
                    debug!(
                        "ルーム \"{}\"の接続数がデクリメントされました。({}→{})",
                        self.name,
                        count,
                        count - 1
                    );
                    *status = Active(count - 1);
                }
            }
            Inactive(_) => {
                warn!(
                    "既に削除待機中であるルーム \"{}\"に対してカウントのデクリメントを試行しました。",
                    self.name
                );
            }
        }
    }
}

#[derive(Debug)]
enum RoomStatus {
    Active(usize),
    Inactive(JoinHandle<()>),
}
