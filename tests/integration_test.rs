use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{delete, get, post},
    Router,
};
use tower::ServiceExt;

use rust_distributed_routing::{api, replication, routing, state::AppState};

fn app() -> Router {
    let state = AppState::new("node-a".to_string());

    Router::new()
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
        .with_state(state)
}

#[tokio::test]
async fn health_returns_healthy() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_route_returns_created() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/routes")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"service":"payments","target":"node-a"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_route_rejects_empty_service() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/routes")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"service":"","target":"node-a"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_route_rejects_invalid_target() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/routes")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"service":"payments","target":"node-x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_missing_route_returns_not_found() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/routes/payments")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn replication_accepts_route_update() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"service":"payments","target":"node-a","version":1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn replication_ignores_stale_update() {
    let router = app();

    let first = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"service":"payments","target":"node-b","version":2}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first.status(), StatusCode::OK);

    let second = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"service":"payments","target":"node-a","version":1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second.status(), StatusCode::OK);

    let body = axum::body::to_bytes(second.into_body(), usize::MAX)
        .await
        .unwrap();

    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("stale update"));
    assert!(body.contains("current_version"));
}

#[tokio::test]
async fn replication_rejects_invalid_target() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"service":"payments","target":"node-x","version":1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn replication_delete_removes_route() {
    let router = app();

    let create = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"service":"payments","target":"node-a","version":1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::OK);

    let delete = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate-delete")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"service":"payments","version":2}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete.status(), StatusCode::OK);

    let routes = router
        .oneshot(
            Request::builder()
                .uri("/routes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(routes.status(), StatusCode::OK);

    let body = axum::body::to_bytes(routes.into_body(), usize::MAX)
        .await
        .unwrap();

    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(!body.contains("payments"));
}

#[tokio::test]
async fn replication_delete_ignores_stale_delete() {
    let router = app();

    let create = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"service":"payments","target":"node-b","version":5}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::OK);

    let delete = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/replicate-delete")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"service":"payments","version":4}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete.status(), StatusCode::OK);

    let body = axum::body::to_bytes(delete.into_body(), usize::MAX)
        .await
        .unwrap();

    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("stale delete"));
    assert!(body.contains("current_version"));
}
