use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::replication::RouteUpdate;
use crate::state::{AppState, RouteEntry};

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

pub async fn get_routes(State(state): State<AppState>) -> Json<RoutesResponse> {
    let routes = state.routes.read().await;

    let routes = routes
        .iter()
        .map(|(service, entry)| {
            (
                service.clone(),
                RouteResponse {
                    target: entry.target.clone(),
                    version: entry.version,
                },
            )
        })
        .collect();

    Json(RoutesResponse {
        node: state.node_id.clone(),
        routes,
    })
}

pub async fn create_route(
    State(state): State<AppState>,
    Json(request): Json<CreateRouteRequest>,
) -> Result<(StatusCode, Json<RoutesResponse>), (StatusCode, Json<ErrorResponse>)> {
    if request.service.trim().is_empty() {
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
        service: request.service.clone(),
        target: request.target.clone(),
        version,
    };

    {
        let mut routes = state.routes.write().await;

        routes.insert(
            request.service.clone(),
            RouteEntry {
                target: request.target.clone(),
                version,
            },
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

    let routes = routes
        .iter()
        .map(|(service, entry)| {
            (
                service.clone(),
                RouteResponse {
                    target: entry.target.clone(),
                    version: entry.version,
                },
            )
        })
        .collect();

    let response = RoutesResponse {
        node: state.node_id.clone(),
        routes,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_route(
    State(state): State<AppState>,
    Path(service): Path<String>,
) -> StatusCode {
    let mut routes = state.routes.write().await;

    if routes.remove(&service).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
