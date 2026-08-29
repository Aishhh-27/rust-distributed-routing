# Rust Distributed Routing

A small TCP routing project written in Rust.

I built this to get some hands-on experience with routing connections between multiple backend servers and handling the networking side with Tokio.

## How it works

The router listens for incoming TCP connections on port `7000` and forwards them to one of the backend servers.

```text
                    Client
                       |
                       v
                 Router :7000
                 /     |     \
                /      |      \
               v       v       v
           :8001    :8002    :8003
          Backend  Backend  Backend
```

The backend addresses are configured locally. When a connection comes in, the router selects a backend and connects the client to it.

## What I worked on

* TCP connections using Rust
* Async networking with Tokio
* Handling multiple connections
* Routing connections between backend servers
* Backend connection errors
* Basic failure handling

## Running it

Build the project:

```bash
cargo build
```

Run the router:

```bash
cargo run
```

The router listens on:

```text
127.0.0.1:7000
```

The backend servers can be run on:

```text
127.0.0.1:8001
127.0.0.1:8002
127.0.0.1:8003
```

Once the router and backends are running, a TCP client can connect to the router:

```bash
nc 127.0.0.1 7000
```

## Testing

Run the tests with:

```bash
cargo test
```

I also use these while working on the project:

```bash
cargo check
cargo fmt
cargo clippy
```

## Project structure

```text
rust-distributed-routing/
├── src/
│   └── main.rs
├── Cargo.toml
├── Cargo.lock
└── README.md
```

## Why I built it

I wanted a small project where I could work directly with TCP and asynchronous Rust instead of hiding the networking behind a higher-level framework.

It also gave me a chance to experiment with what happens when a backend is unavailable and how the router should handle that without taking down the whole process.

## Next things I would add

If I continue expanding this project, I'd like to add:

* backend health checks
* configurable routing strategies
* connection timeouts
* better retry handling
* metrics and structured logging
* graceful shutdown

This is intentionally a small project, but it gives me a useful base for experimenting with more advanced routing and proxy behaviour.
