mod api;
mod failure_detection;
mod membership;
mod node;
mod replication;
mod routing;
mod state;

use std::env;
use std::net::SocketAddr;

use axum::{
    routing::{delete, get, post},
    Router,
};

use state::{AppState, RouteState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let node_id = env::var("NODE_ID").unwrap_or_else(|_| "node-a".to_string());

    let port = env::var("PORT")
        .unwrap_or_else(|_| "7100".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    let state = AppState::new(node_id.clone());

    let app = Router::new()
        .route("/health", get(api::health))
        .route("/nodes", get(api::get_nodes))
        .route("/internal/state", get(api::get_state))
        .route("/internal/replicate", post(replication::replicate_route))
        .route(
            "/internal/replicate-delete",
            post(replication::replicate_delete),
        )
        .route(
            "/routes",
            get(routing::get_routes).post(routing::create_route),
        )
        .route("/routes/{service}", delete(routing::delete_route))
        .with_state(state.clone());

    let address = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Starting {node_id} on {address}");

    tokio::spawn(failure_detection::start(state.clone()));

    tokio::spawn(failure_detection::start(state.clone()));

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind");

    /*
     * State recovery.
     *
     * When a node starts, ask the other known nodes for
     * their current state and install newer route states.
     *
     * Tombstones are recovered too, so a deleted route
     * cannot reappear after a node restarts.
     */
    if node_id != "node-a" {
        let nodes = state.nodes.read().await.clone();
        let client = reqwest::Client::new();

        for node in nodes {
            if node.id == node_id {
                continue;
            }

            let url = format!("{}/internal/state", node.address);

            println!("Attempting state recovery from {}", node.id);

            match client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        eprintln!(
                            "State recovery from {} failed with status {}",
                            node.id,
                            response.status()
                        );
                        continue;
                    }

                    match response.json::<api::StateResponse>().await {
                        Ok(remote_state) => {
                            let mut routes = state.routes.write().await;

                            let mut recovered = 0;

                            for (service, remote_entry) in remote_state.routes {
                                let remote_version = match &remote_entry {
                                    RouteState::Active(entry) => entry.version,
                                    RouteState::Deleted(tombstone) => tombstone.version,
                                };

                                let should_update = match routes.get(&service) {
                                    Some(local_entry) => {
                                        let local_version = match local_entry {
                                            RouteState::Active(entry) => entry.version,
                                            RouteState::Deleted(tombstone) => tombstone.version,
                                        };

                                        remote_version > local_version
                                    }
                                    None => true,
                                };

                                if should_update {
                                    routes.insert(service, remote_entry);
                                    recovered += 1;
                                }
                            }

                            println!("Recovered {} route state(s) from {}", recovered, node.id);
                        }

                        Err(error) => {
                            eprintln!("Failed to decode state from {}: {}", node.id, error);
                        }
                    }
                }

                Err(error) => {
                    eprintln!("Could not reach {} for state recovery: {}", node.id, error);
                }
            }
        }
    }

    axum::serve(listener, app).await.expect("server error");
}
