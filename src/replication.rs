use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use serde::{Deserialize, Serialize};

use crate::state::{AppState, RouteEntry, RouteState, Tombstone};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteUpdate {
    pub service: String,
    pub target: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUpdate {
    pub service: String,
    pub version: u64,
}

fn route_version(route: &RouteState) -> u64 {
    match route {
        RouteState::Active(entry) => entry.version,
        RouteState::Deleted(tombstone) => tombstone.version,
    }
}

pub async fn replicate_route(
    State(state): State<AppState>,
    Json(update): Json<RouteUpdate>,
) -> impl IntoResponse {
    if update.service.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "service cannot be empty"
            })),
        );
    }

    if !matches!(update.target.as_str(), "node-a" | "node-b" | "node-c") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid target node"
            })),
        );
    }

    let mut routes = state.routes.write().await;

    if let Some(existing) = routes.get(&update.service) {
        let current_version = route_version(existing);

        if current_version >= update.version {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ignored",
                    "reason": "stale update",
                    "node": state.node_id,
                    "current_version": current_version,
                    "received_version": update.version
                })),
            );
        }
    }

    routes.insert(
        update.service,
        RouteState::Active(RouteEntry {
            target: update.target,
            version: update.version,
        }),
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "replicated",
            "node": state.node_id,
            "version": update.version
        })),
    )
}

pub async fn replicate_delete(
    State(state): State<AppState>,
    Json(update): Json<DeleteUpdate>,
) -> impl IntoResponse {
    if update.service.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "service cannot be empty"
            })),
        );
    }

    let mut routes = state.routes.write().await;

    if let Some(existing) = routes.get(&update.service) {
        let current_version = route_version(existing);

        if current_version >= update.version {
            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "ignored",
                    "reason": "stale delete",
                    "node": state.node_id,
                    "current_version": current_version,
                    "received_version": update.version
                })),
            );
        }
    }

    routes.insert(
        update.service,
        RouteState::Deleted(Tombstone {
            version: update.version,
        }),
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "deleted",
            "node": state.node_id,
            "version": update.version
        })),
    )
}
