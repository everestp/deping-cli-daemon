# DePing CLI Miner

A high-performance DePIN (Decentralized Physical Infrastructure Network) node designed for distributed uptime monitoring, latency profiling, and internet infrastructure observability.

The miner operates as a lightweight background daemon that securely receives monitoring tasks from the DePing network, executes network measurements, and streams cryptographically verifiable telemetry back to the network.

---

## Overview

DePing transforms internet monitoring into a decentralized infrastructure network.

Instead of relying on a small number of centralized monitoring servers, DePing distributes monitoring workloads across independent node operators around the world.

Each miner performs network measurements from its own geographic and network location, producing globally distributed uptime and latency insights.

---

## Architecture
# DePing Network Architecture

## High-Level System Architecture

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                            DePing Network                                 │
└────────────────────────────────────────────────────────────────────────────┘

                                 ┌───────────────┐
                                 │   Customers   │
                                 │ Dashboards    │
                                 │ APIs          │
                                 └───────┬───────┘
                                         │
                                         ▼
                         ┌─────────────────────────────┐
                         │    Go Backend Platform      │
                         ├─────────────────────────────┤
                         │ REST API (Gin)             │
                         │ Authentication Service     │
                         │ Reward Engine              │
                         │ Task Generator             │
                         │ Metrics Aggregator         │
                         └─────────────┬──────────────┘
                                       │
                                       │ gRPC HTTP/2
                                       ▼
                    ┌────────────────────────────────────┐
                    │      gRPC Ingress Core             │
                    │ Persistent Task Streaming          │
                    │ Bi-Directional Streams            │
                    └──────────────┬─────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                    │
              ▼                    ▼                    ▼

      ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
      │ Rust Miner A │    │ Rust Miner B │    │ Rust Miner C │
      └──────────────┘    └──────────────┘    └──────────────┘

              │                    │                    │
              └────────────────────┼────────────────────┘
                                   │
                                   ▼

                    Distributed Internet Monitoring
```

---

# Miner Internal Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                           RUST CLIENT MINER                            │
└─────────────────────────────────────────────────────────────────────────┘

    Setup Phase
         │
         ▼

┌──────────────────────┐
│ Identity Manager     │
├──────────────────────┤
│ Load Keypair         │
│ Generate Keypair     │
│ Store Identity       │
└──────────┬───────────┘
           │
           ▼

┌──────────────────────┐
│ Authentication Plane │
├──────────────────────┤
│ Challenge Request    │
│ Ed25519 Signature    │
│ Session Validation   │
└──────────┬───────────┘
           │
           ▼

┌──────────────────────┐
│ gRPC Stream Engine   │
├──────────────────────┤
│ HTTP/2 Stream        │
│ Heartbeats           │
│ Auto Reconnect       │
│ Backoff Engine       │
└──────────┬───────────┘
           │
           ▼

┌──────────────────────┐
│ Task Scheduler       │
├──────────────────────┤
│ MPSC Queue           │
│ Semaphore Limits     │
│ Target Isolation     │
└──────────┬───────────┘
           │
           ▼

┌──────────────────────┐
│ Execution Workers    │
├──────────────────────┤
│ DNS Profiling        │
│ TCP Profiling        │
│ TLS Profiling        │
│ TTFB Profiling       │
└──────────┬───────────┘
           │
           ▼

┌──────────────────────┐
│ Metrics Encoder      │
├──────────────────────┤
│ Protobuf Serialization│
│ Binary Compression   │
└──────────┬───────────┘
           │
           ▼

      Back To gRPC Stream
```

---

# Authentication Flow

```text
Miner Startup
      │
      ▼

┌─────────────────┐
│ Load Keypair    │
└────────┬────────┘
         │
         ▼

┌─────────────────┐
│ Request Challenge│
└────────┬────────┘
         │
         ▼

┌─────────────────┐
│ Server Challenge│
│ Random Nonce    │
└────────┬────────┘
         │
         ▼

┌─────────────────┐
│ Sign Challenge  │
│ Ed25519         │
└────────┬────────┘
         │
         ▼

┌─────────────────┐
│ Verify Signature│
└────────┬────────┘
         │
         ▼

┌─────────────────┐
│ Issue Session   │
└────────┬────────┘
         │
         ▼

   Open gRPC Stream
```

---

# Stream Engine Lifecycle

```text
┌─────────────────────┐
│ Connect gRPC Server │
└──────────┬──────────┘
           │
           ▼

┌─────────────────────┐
│ Stream Established  │
└──────────┬──────────┘
           │
           ▼

┌─────────────────────┐
│ Receive Jobs        │
└──────────┬──────────┘
           │
           ▼

┌─────────────────────┐
│ Send Heartbeats     │
│ Every 15-30 Seconds │
└──────────┬──────────┘
           │
           ▼

      Connection Lost?
           │
      ┌────┴────┐
      │         │
     NO        YES
      │         │
      ▼         ▼

 Continue   Exponential Backoff
                 │
                 ▼

          Reconnect Attempt
                 │
                 ▼

          Stream Recovered
```

---

# Scheduler Architecture

```text
Incoming Jobs
      │
      ▼

┌─────────────────────┐
│ MPSC Job Queue      │
└──────────┬──────────┘
           │
           ▼

┌─────────────────────┐
│ Semaphore Guard     │
│ Max Workers = 10    │
└──────────┬──────────┘
           │
           ▼

     Permit Available?
           │
      ┌────┴────┐
      │         │
     NO        YES
      │         │
      ▼         ▼

 Wait Queue   Spawn Worker
                    │
                    ▼

        ┌─────────────────────┐
        │ Active Target Map   │
        └──────────┬──────────┘
                   │
                   ▼

        Same Host Already Active?
                   │
             ┌─────┴─────┐
             │           │
            YES         NO
             │           │
             ▼           ▼

        Delay Task   Execute
```

---

# Worker Execution Pipeline

```text
                Start Check
                     │
                     ▼

       ┌─────────────────────────┐
       │ DNS Resolution          │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ TCP Handshake           │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ TLS Handshake           │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ HTTP Request            │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ Time To First Byte      │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ Response Validation     │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ Build Metrics Payload   │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ Protobuf Encoding       │
       └────────────┬────────────┘
                    │
                    ▼

       ┌─────────────────────────┐
       │ Send Result To Network  │
       └─────────────────────────┘
```

---

# Rust Project Structure

```text
deping/
│
├── Cargo.toml
├── build.rs
├── proto/
│   └── monitor.proto
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── identity/
│   │   └── mod.rs
│   │
│   ├── network/
│   │   ├── mod.rs
│   │   └── stream.rs
│   │
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── scheduler.rs
│   │   └── worker.rs
│   │
│   ├── types/
│   │   ├── mod.rs
│   │   ├── metrics.rs
│   │   └── task.rs
│   │
│   └── generated/
│       └── monitor.rs
│
└── .github/
    └── workflows/
        └── release.yml
```


## Core Components

### Identity System

Every miner owns a cryptographic Ed25519 identity.

The private key never leaves the machine.

Authentication occurs through challenge-response signing.

```text
Server → Challenge
Miner  → Sign Challenge
Server → Verify Signature
```

---

### Persistent Stream Engine

The miner maintains a long-lived gRPC connection using HTTP/2.

Features:

* Automatic reconnect
* Exponential backoff
* Heartbeat pings
* Stream recovery
* Low-overhead binary messaging

---

### Task Scheduler

The scheduler protects the machine from overload.

Features:

* Bounded concurrency
* Tokio semaphore protection
* Internal MPSC buffering
* Target isolation logic
* Backpressure handling

Example:

```text
100 incoming jobs

Max concurrency = 10

10 executing
90 waiting
```

---

### Network Execution Engine

Each monitoring task collects detailed telemetry.

Measurement phases:

```text
DNS Resolution
      ↓
TCP Handshake
      ↓
TLS Handshake
      ↓
TTFB
      ↓
Response Validation
```

Collected metrics include:

* DNS latency
* TCP connect latency
* TLS handshake latency
* Time To First Byte (TTFB)
* HTTP status code
* Success/failure state

---

## Security Model

### Private Keys Never Leave Device

The miner only transmits signatures.

Private keys remain local.

### TLS Communication

All network communication occurs over encrypted channels.

### Resource Protection

The miner enforces:

* Request limits
* Concurrency limits
* Timeout boundaries
* Target isolation

### Anti-Abuse Design

The scheduler prevents excessive concurrent requests to the same host.

This ensures nodes behave as monitoring infrastructure rather than traffic amplification tools.

---

## Installation

### Linux

```bash
curl -fsSL https://raw.githubusercontent.com/everestp/deping-cli-daemon/main/install.sh | bash
```

### Manual Installation

Download the latest release binary.

```bash
chmod +x deping-linux-amd64
sudo mv deping-linux-amd64 /usr/local/bin/deping
```

---

## Initialize Node

Generate or load your node identity.

```bash
deping setup
```

Output:

```text
🚀 DePing Node Initialized

🔑 Public Key:
xxxxxxxxxxxxxxxxxxxxxxxx
```

---

## Start Miner

```bash
deping start
```

Example:

```text
INFO Starting DePing Miner Node
INFO Connected to Gateway
INFO Stream Established
INFO Waiting For Jobs
```

---

## Configuration

Environment variables:

```bash
DEPING_GATEWAY_URL=
DEPING_GRPC_ENDPOINT=
DEPING_MAX_CONCURRENT_JOBS=
DEPING_HEARTBEAT_INTERVAL=
DEPING_CONNECT_TIMEOUT=
```

---

## Release Build

```bash
cargo build --release
```

Optimized release profile:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Development

Requirements:

* Rust Stable
* Cargo
* Protocol Buffers Compiler (protoc)

Linux:

```bash
sudo apt install protobuf-compiler
```

MacOS:

```bash
brew install protobuf
```

---

## Roadmap

* Geographic reputation scoring
* Node performance scoring
* Reward distribution engine
* Multi-protocol probes
* TCP probes
* DNS probes
* SSL certificate analytics
* Regional latency maps
* Blockchain reward settlement

---

## Tech Stack

* Rust
* Tokio
* Tonic gRPC
* Reqwest
* Ed25519-Dalek
* Prost
* Tracing
* Protobuf



Built for decentralized internet observability.
