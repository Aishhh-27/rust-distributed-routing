use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::{AppState, RouteEntry};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub node: String,
}

#[derive(Debug, Serialize)]
pub struct NodeInfoResponse {
    pub id: String,
    pub address: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateResponse {
    pub routes: HashMap<String, RouteEntry>,
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            node: state.node_id.clone(),
        }),
    )
}

pub async fn get_nodes(State(state): State<AppState>) -> Json<Vec<NodeInfoResponse>> {
    let nodes = state.nodes.read().await.clone();

    Json(
        nodes
            .into_iter()
            .map(|node| NodeInfoResponse {
                id: node.id,
                address: node.address,
                status: node.status,
            })
            .collect(),
    )
}

pub async fn get_state(State(state): State<AppState>) -> Json<StateResponse> {
    let routes = state.routes.read().await.clone();

    Json(StateResponse { routes })
}
