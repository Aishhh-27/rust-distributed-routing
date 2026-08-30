use std::time::Duration;

use reqwest::Client;

use crate::state::AppState;

pub async fn start(state: AppState) {
    let client = Client::new();

    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let nodes = state.nodes.read().await.clone();

        for node in nodes {
            if node.id == state.node_id {
                continue;
            }

            let url = format!("{}/health", node.address);

            let healthy = client
                .get(&url)
                .timeout(Duration::from_secs(1))
                .send()
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false);

            let mut current_nodes = state.nodes.write().await;

            if let Some(current) = current_nodes
                .iter_mut()
                .find(|current| current.id == node.id)
            {
                current.status = if healthy {
                    "healthy".to_string()
                } else {
                    "unhealthy".to_string()
                };
            }
        }
    }
}
