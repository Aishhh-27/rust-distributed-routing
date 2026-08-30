# Rust Distributed Routing

A small distributed routing service I built in Rust using Axum, Tokio, and Reqwest.

The project runs multiple routing nodes that keep route information replicated between them. Each node can detect unhealthy peers and recover route state when it starts.

## What it does

- Runs multiple routing nodes on different ports
- Creates and deletes service routes
- Replicates route updates between nodes
- Uses monotonically increasing versions to ignore stale updates
- Keeps tombstones for deleted routes
- Recovers route state when a node starts
- Detects unhealthy nodes through periodic health checks
- Exposes HTTP endpoints for health, membership, routes, and internal state

## Architecture

The cluster contains three routing nodes:

    node-a :7100
       |
       +------------------+
       |                  |
       v                  v
    node-b :7101       node-c :7102

Each node maintains a local copy of the routing state.

When a route is created or updated:

1. The local node assigns a new version.
2. The route is stored locally.
3. The update is replicated to the other nodes.
4. Other nodes compare the received version with their current version.
5. Older updates are ignored.

## Routing State

Each route contains:

- service name
- target node
- version number

Example:

    {
      "service": "checkout",
      "target": "node-c",
      "version": 2
    }

Versions provide a simple way to prevent stale route updates from overwriting newer state.

## Route Tombstones

Deleted routes are represented using tombstones instead of immediately forgetting their version.

For example, if a route is deleted at version 4, the node keeps a deletion record containing version 4.

This prevents an older replicated update from recreating the deleted route.

The same tombstone state is also included during state recovery, so a restarted node does not accidentally restore an old route.

## State Recovery

When a node starts, it attempts to recover routing state from its peers.

For example:

    Starting node-b on 0.0.0.0:7101
    Attempting state recovery from node-a
    Recovered 3 route(s) from node-a
    Attempting state recovery from node-c
    Recovered 1 route(s) from node-c

The node compares route versions while recovering state and keeps the newest known state.

This gives a restarted node a way to rebuild its routing table without requiring a separate persistent database.

## Failure Detection

Each node periodically checks the health endpoint of the other nodes.

The health checker:

- runs every few seconds
- sends a request to each peer
- uses a request timeout
- marks the peer healthy when the request succeeds
- marks the peer unhealthy when the request fails

Example membership response:

    [
      {
        "id": "node-a",
        "address": "http://127.0.0.1:7100",
        "status": "healthy"
      },
      {
        "id": "node-b",
        "address": "http://127.0.0.1:7101",
        "status": "healthy"
      },
      {
        "id": "node-c",
        "address": "http://127.0.0.1:7102",
        "status": "healthy"
      }
    ]

This is intentionally simple. It is a lightweight failure detector rather than a full consensus or membership protocol.

## HTTP API

### Health

    GET /health

Returns the health status of the current node.

Example:

    {
      "status": "healthy",
      "node": "node-a"
    }

### Membership

    GET /nodes

Returns the known nodes and their current health status.

### List Routes

    GET /routes

Returns the routes currently visible on the node.

Example:

    {
      "node": "node-a",
      "routes": {
        "checkout": {
          "target": "node-c",
          "version": 2
        },
        "payments": {
          "target": "node-a",
          "version": 3
        }
      }
    }

### Create or Update a Route

    POST /routes

Example request:

    curl -X POST http://127.0.0.1:7100/routes       -H "Content-Type: application/json"       -d service:checkout

The local node creates a new version and replicates the route update.

### Delete a Route

    DELETE /routes/{service}

The route is removed from the active routing table and a tombstone is created with the new version.

### Replication

Internal replication endpoints are used by the nodes to exchange route updates and deletion information.

Replication validates incoming route data and ignores stale updates.

## Running the Cluster

Build the project:

    cargo build

Start node-a:

    NODE_ID=node-a PORT=7100 cargo run

Start node-b in another terminal:

    NODE_ID=node-b PORT=7101 cargo run

Start node-c in another terminal:

    NODE_ID=node-c PORT=7102 cargo run

The nodes use the default local cluster configuration:

    node-a -> http://127.0.0.1:7100
    node-b -> http://127.0.0.1:7101
    node-c -> http://127.0.0.1:7102

## Testing

Run formatting:

    cargo fmt

Check the project:

    cargo check

Run the test suite:

    cargo test

The integration tests cover:

- creating valid routes
- rejecting empty service names
- rejecting invalid target nodes
- deleting missing routes
- health checks
- accepting replicated route updates
- rejecting invalid replicated targets
- ignoring stale route updates
- deleting replicated routes
- ignoring stale replicated deletes

## Manual Verification

After starting all three nodes, check their health:

    for port in 7100 7101 7102; do
      echo "===== PORT $port ====="
      curl -s http://127.0.0.1:$port/health
      echo
    done

Check the routing state:

    for port in 7100 7101 7102; do
      echo "===== PORT $port ====="
      curl -s http://127.0.0.1:$port/routes
      echo
    done

Create a route on node-a:

    curl -X POST http://127.0.0.1:7100/routes       -H "Content-Type: application/json"       -d service:checkout

Then check node-b and node-c:

    curl http://127.0.0.1:7101/routes
    curl http://127.0.0.1:7102/routes

The new route should appear on the other nodes after replication.

## Example Cluster State

A healthy cluster can look like:

    node-a :7100
      checkout -> node-c (version 2)
      payments -> node-a (version 3)
      service-b -> node-c (version 1)

    node-b :7101
      checkout -> node-c (version 2)
      payments -> node-a (version 3)
      service-a -> node-c (version 3)
      service-b -> node-c (version 1)

    node-c :7102
      checkout -> node-c (version 2)
      payments -> node-a (version 3)
      service-a -> node-c (version 3)
      service-b -> node-c (version 1)

The ordering of routes may differ because they are stored in a HashMap.

## Project Structure

    rust-distributed-routing/
    |
    +-- src/
    |   +-- api.rs
    |   +-- failure_detection.rs
    |   +-- membership.rs
    |   +-- replication.rs
    |   +-- routing.rs
    |   +-- state.rs
    |   +-- lib.rs
    |   +-- main.rs
    |
    +-- tests/
    |   +-- integration_test.rs
    |
    +-- Cargo.toml
    +-- Cargo.lock
    +-- README.md

## Design Notes

This project intentionally keeps the distributed system small and understandable.

It does not attempt to implement consensus, leader election, durable storage, or a production-grade membership protocol.

Instead, it focuses on a few core distributed-systems ideas:

- replicated state
- version-based conflict handling
- deletion tombstones
- state recovery
- peer health detection
- asynchronous HTTP communication
- concurrent shared state

The implementation uses Tokio for asynchronous execution, Axum for the HTTP API, Reqwest for node-to-node communication, and shared RwLocks for concurrent in-memory state.

## Dependencies

Main dependencies include:

- Rust
- Tokio
- Axum
- Reqwest
- Serde
- Serde JSON

## Verification

The current test suite passes successfully:

    running 10 tests

    test create_route_rejects_empty_service ... ok
    test create_route_rejects_invalid_target ... ok
    test delete_missing_route_returns_not_found ... ok
    test health_returns_healthy ... ok
    test replication_accepts_route_update ... ok
    test replication_rejects_invalid_target ... ok
    test replication_ignores_stale_update ... ok
    test replication_delete_removes_route ... ok
    test replication_delete_ignores_stale_delete ... ok
    test create_route_returns_created ... ok

    test result: ok. 10 passed; 0 failed

## Status

This is a small learning project focused on distributed routing concepts and practical Rust networking.

The current implementation supports replicated route state, version-based conflict handling, deletion tombstones, startup state recovery, and basic peer failure detection.
