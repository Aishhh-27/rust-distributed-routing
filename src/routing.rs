use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::replication::{DeleteUpdate, RouteUpdate};
use crate::state::{AppState, RouteEntry, RouteState, Tombstone};

#[derive(Debug, Deserialize)]
pub struct CreateRouteRequest {
    pub service: String,
    pub target: String,
}

#[derive(Debug, Serialize)]
pub struct RoutesResponse {
    pub node: String,
    pub routes: HashMap<String, RouteResponse>,
}

#[derive(Debug, Serialize)]
pub struct RouteResponse {
    pub target: String,
    pub version: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn route_version(route: &RouteState) -> u64 {
    match route {
        RouteState::Active(entry) => entry.version,
        RouteState::Deleted(tombstone) => tombstone.version,
    }
}

fn active_routes(routes: &HashMap<String, RouteState>) -> HashMap<String, RouteResponse> {
    routes
        .iter()
        .filter_map(|(service, state)| match state {
            RouteState::Active(entry) => Some((
                service.clone(),
                RouteResponse {
                    target: entry.target.clone(),
                    version: entry.version,
                },
            )),
            RouteState::Deleted(_) => None,
        })
        .collect()
}

pub async fn get_routes(State(state): State<AppState>) -> Json<RoutesResponse> {
    let routes = state.routes.read().await;

    Json(RoutesResponse {
        node: state.node_id.clone(),
        routes: active_routes(&routes),
    })
}

pub async fn create_route(
    State(state): State<AppState>,
    Json(request): Json<CreateRouteRequest>,
) -> Result<(StatusCode, Json<RoutesResponse>), (StatusCode, Json<ErrorResponse>)> {
    let service = request.service.trim().to_string();

    if service.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "service cannot be empty".to_string(),
            }),
        ));
    }

    if !matches!(request.target.as_str(), "node-a" | "node-b" | "node-c") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "unknown target node".to_string(),
            }),
        ));
    }

    let version = state.next_version().await;

    let update = RouteUpdate {
        service: service.clone(),
        target: request.target.clone(),
        version,
    };

    {
        let mut routes = state.routes.write().await;

        routes.insert(
            service.clone(),
            RouteState::Active(RouteEntry {
                target: request.target.clone(),
                version,
            }),
        );
    }

    let nodes = state.nodes.read().await.clone();
    let client = reqwest::Client::new();

    for node in nodes {
        if node.id == state.node_id {
            continue;
        }

        let url = format!("{}/internal/replicate", node.address);

        match client.post(&url).json(&update).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    println!(
                        "Replicated route '{}' version {} to {}",
                        update.service, update.version, node.id
                    );
                } else {
                    eprintln!(
                        "Replication to {} failed with status {}",
                        node.id,
                        response.status()
                    );
                }
            }

            Err(error) => {
                eprintln!("Replication to {} failed: {}", node.id, error);
            }
        }
    }

    let routes = state.routes.read().await;

    let response = RoutesResponse {
        node: state.node_id.clone(),
        routes: active_routes(&routes),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_route(
    State(state): State<AppState>,
    Path(service): Path<String>,
) -> StatusCode {
    let version = state.next_version().await;

    {
        let mut routes = state.routes.write().await;

        match routes.get(&service) {
            Some(existing) => {
                if route_version(existing) >= version {
                    return StatusCode::NOT_FOUND;
                }
            }
            None => {
                return StatusCode::NOT_FOUND;
            }
        }

        routes.insert(service.clone(), RouteState::Deleted(Tombstone { version }));
    }

    let update = DeleteUpdate {
        service: service.clone(),
        version,
    };

    let nodes = state.nodes.read().await.clone();
    let client = reqwest::Client::new();

    for node in nodes {
        if node.id == state.node_id {
            continue;
        }

        let url = format!("{}/internal/replicate-delete", node.address);

        match client.post(&url).json(&update).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    println!(
                        "Replicated delete '{}' version {} to {}",
                        update.service, update.version, node.id
                    );
                } else {
                    eprintln!(
                        "Delete replication to {} failed with status {}",
                        node.id,
                        response.status()
                    );
                }
            }

            Err(error) => {
                eprintln!("Delete replication to {} failed: {}", node.id, error);
            }
        }
    }

    StatusCode::NO_CONTENT
}
