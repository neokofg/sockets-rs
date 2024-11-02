use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelAuth {
    pub user_id: u64,
    pub channel_name: String,
    exp: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChannelMessage {
    channel: String,
    event: String,
    data: String,
}

pub struct ConnectionManager {
    channels: HashMap<String, broadcast::Sender<ChannelMessage>>,
    users: HashMap<u64, Vec<String>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            users: HashMap::new(),
        }
    }

    pub(crate) async fn subscribe(&mut self, channel_name: String, user_id: u64) -> broadcast::Receiver<ChannelMessage> {
        if !self.channels.contains_key(&channel_name) {
            let (tx, _) = broadcast::channel(100);
            self.channels.insert(channel_name.clone(), tx);
        }

        self.users.entry(user_id)
            .or_insert_with(Vec::new)
            .push(channel_name.clone());

        self.channels.get(&channel_name)
            .unwrap()
            .subscribe()
    }
}