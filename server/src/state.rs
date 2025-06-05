use std::{collections::{HashMap, VecDeque}, ops::Deref, sync::Arc};

use axum::extract::ws::Message;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

const MAX_HISTORY_SIZE: usize = 100;
const REMOVE_AFTER: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone)]
pub struct AppState(Arc<RwLock<HashMap<String, Room>>>);
impl AppState {
    pub fn new() -> Self {
        AppState(Arc::new(RwLock::new(HashMap::new())))
    }
    pub fn new_room(&self, name: &str) -> Room {
        let (broadcaster, _) = broadcast::channel(MAX_HISTORY_SIZE);
        Room {
            name: name.to_string(),
            status: Arc::new(RwLock::new(RoomStatus::Active(0))),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY_SIZE))),
            broadcaster,
            parent: Arc::new(self.clone()),
        }
    }

}
impl Deref for AppState {
    type Target = Arc<RwLock<HashMap<String, Room>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Room {
    name: String,
    status: Arc<RwLock<RoomStatus>>,
    history: Arc<RwLock<VecDeque<String>>>,
    broadcaster: broadcast::Sender<Message>,
    parent: Arc<AppState>,
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
    pub async fn connection_count(&self) -> usize {
        if let RoomStatus::Active(count) = *self.status.read().await {
            count
        } else {
            0
        }
    }
    pub async fn increment_connection(&self) {
        let mut status = self.status.write().await;
        *status = RoomStatus::Active(match &*status {
            RoomStatus::Active(count) => {
                count + 1
            }
            RoomStatus::Inactive(token) => {
                token.cancel();
                1
            }
        });
    }
    pub async fn decrement_connection(&self) {
        let mut status = self.status.write().await;
        if let RoomStatus::Active(count) = *status {
            *status = RoomStatus::Active(count - 1);
            if count == 1 {
                let token = CancellationToken::new();
                *status = RoomStatus::Inactive(token.clone());
                let room_name = self.name.clone();
                let parent = self.parent.clone();
                // ルーム削除のタスク。parent.write().await も待つことで、ws_handlerと競合して
                // HashMapから削除されたルームが生存してしまうことを防ぐ
                tokio::spawn(async move {
                    tokio::select! {
                        _ = token.cancelled() => {},
                        _ = async {
                            tokio::time::sleep(REMOVE_AFTER).await;
                            parent.write().await.remove(&room_name);
                        } => {}
                    }
                });
            }
        }
    }
}

enum RoomStatus {
    Active(usize),
    Inactive(CancellationToken),
}
