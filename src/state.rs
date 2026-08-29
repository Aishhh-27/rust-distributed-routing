use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::membership::{default_membership, NodeInfo};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteEntry {
    pub target: String,
    pub version: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub node_id: String,
    pub routes: Arc<RwLock<HashMap<String, RouteEntry>>>,
    pub nodes: Arc<RwLock<Vec<NodeInfo>>>,
    pub version: Arc<RwLock<u64>>,
}

impl AppState {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            routes: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(default_membership())),
            version: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn next_version(&self) -> u64 {
        let mut version = self.version.write().await;

        *version += 1;

        *version
    }
}
