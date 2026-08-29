# Rust Distributed Routing

A small distributed routing service written in Rust.

I built this to experiment with keeping routing information synchronized between multiple nodes. Each node keeps its own local state, and route changes are replicated to the other nodes.

## What it does

The service currently supports:

- creating and deleting routes
- replicating route changes between nodes
- versioning route updates
- ignoring stale updates
- replicating deletions
- recovering route state when a node starts
- basic node health and membership endpoints
- integration tests for the main routing and replication behavior

For example, a route can look like:

```json
{
  "payments": {
    "target": "node-a",
    "version": 1
  }
}
