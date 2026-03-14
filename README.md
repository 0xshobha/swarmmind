# 🧠 SwarmMind — Decentralized AI Agent Economy

> **Vertex Swarm Challenge 2026 · Track 3: The Agent Economy + Warm-Up**

A leaderless, peer-to-peer agent marketplace where AI agents autonomously discover each other, negotiate tasks, bid on work, and coordinate execution — all through **Tashi Vertex BFT consensus** with **zero central orchestrator**.

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    SwarmMind Agent Economy                       │
│                                                                 │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────────┐ │
│  │  Agent   │  │  Task    │  │ Auction  │  │   Reputation    │ │
│  │ Identity │  │ Manager  │  │ Engine   │  │    Engine       │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬────────┘ │
│       │             │             │                  │          │
│  ┌────▼─────────────▼─────────────▼──────────────────▼────────┐ │
│  │              Vertex Core (P2P Coordination)                │ │
│  │  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────┐ │ │
│  │  │Handshake │ │ Heartbeat │ │  State   │ │  Recovery    │ │ │
│  │  │ (HELLO)  │ │  (3s)     │ │ Replica  │ │ (Self-Heal)  │ │ │
│  │  └──────────┘ └───────────┘ └──────────┘ └──────────────┘ │ │
│  └────────────────────────┬───────────────────────────────────┘ │
│                           │                                     │
│  ┌────────────────────────▼───────────────────────────────────┐ │
│  │            Tashi Vertex BFT Consensus Engine               │ │
│  │     Sub-100ms · Byzantine Fault Tolerant · Leaderless      │ │
│  │           DAG + Virtual Voting + Fair Ordering             │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## ✨ Key Features

| Feature | Description |
|---------|-------------|
| **🔨 Sealed-Bid Auctions** | Tasks auctioned via Vertex – fair ordering prevents front-running |
| **🏆 Deterministic Allocation** | All nodes compute the same winner from consensus-ordered bids |
| **⭐ Reputation System** | BFT-replicated merit scores influence future auction outcomes |
| **🔄 Self-Healing** | Failed agents' tasks auto-reassigned to healthy peers |
| **💓 Stateful Handshake** | HELLO + heartbeats + stale detection + recovery (Warm-Up) |
| **📊 Live Dashboard** | Real-time swarm topology, task pipeline, and consensus metrics |

## 🚀 Quick Start

### Prerequisites

- **Rust** toolchain ([rustup.rs](https://rustup.rs))
- **CMake** ≥ 4.0 (`pip install cmake`)

### 1. Generate Keypairs

```bash
# Generate keypairs for each agent
cargo run --bin swarmmind-agent -- gen-key   # Agent A
cargo run --bin swarmmind-agent -- gen-key   # Agent B
```

### 2. Start Agents

```bash
# Terminal 1 — Agent Alpha
cargo run --bin swarmmind-agent -- run \
  --bind 127.0.0.1:9000 \
  --secret <SECRET_A> \
  --name Alpha \
  --capabilities compute,reasoning \
  --peer-addr 127.0.0.1:9001 \
  --peer-pubkey <PUBKEY_B> \
  --demo

# Terminal 2 — Agent Beta
cargo run --bin swarmmind-agent -- run \
  --bind 127.0.0.1:9001 \
  --secret <SECRET_B> \
  --name Beta \
  --capabilities data-analysis,natural-language \
  --peer-addr 127.0.0.1:9000 \
  --peer-pubkey <PUBKEY_A>
```

### 3. Submit Tasks

```bash
cargo run --bin swarmmind-agent -- submit-task \
  --bind 127.0.0.1:9002 \
  --secret <SECRET_C> \
  --description "Analyze swarm telemetry data" \
  --capabilities data-analysis \
  --complexity 5 \
  --reward 100 \
  --peer-addr 127.0.0.1:9000 \
  --peer-pubkey <PUBKEY_A> \
  --peer-addr 127.0.0.1:9001 \
  --peer-pubkey <PUBKEY_B>
```

### 4. View Dashboard

Open `dashboard/index.html` in your browser for real-time swarm visualization.

## 📁 Project Structure

```
swarmmind/
├── Cargo.toml                        # Workspace manifest
├── crates/
│   ├── vertex-core/                  # Core P2P layer
│   │   └── src/
│   │       ├── identity.rs           # Agent identity & capabilities
│   │       ├── protocol.rs           # Wire protocol (all message types)
│   │       ├── node.rs               # Vertex engine wrapper
│   │       ├── handshake.rs          # HELLO + heartbeats + stale detection
│   │       └── state.rs              # Replicated swarm state
│   ├── agent-economy/                # Track 3: Agent Economy
│   │   └── src/
│   │       ├── task.rs               # Task broadcasting
│   │       ├── auction.rs            # Sealed-bid auction protocol
│   │       ├── allocation.rs         # Deterministic task allocation
│   │       ├── execution.rs          # Task execution tracking
│   │       ├── reputation.rs         # BFT-replicated reputation
│   │       ├── recovery.rs           # Self-healing & task reassignment
│   │       └── economy.rs            # Economy coordinator
│   └── swarmmind-agent/              # Binary
│       └── src/main.rs               # CLI entry point
└── dashboard/                        # Web monitoring
    ├── index.html
    ├── style.css
    └── app.js
```

## 🎯 Hackathon Tracks Covered

### Warm-Up: Stateful Handshake ✅
- [x] Signed HELLO transaction on startup
- [x] Periodic heartbeats (3s interval)
- [x] Replicated state: `{ peer_id, last_seen_ms, role, status }`
- [x] Role change mirrored in <1 second
- [x] Stale detection (>10s without heartbeat)
- [x] Recovery on peer return

### Track 3: The Agent Economy ✅
- [x] Leaderless task negotiation (no orchestrator)
- [x] Decentralized sealed-bid auctions via Vertex
- [x] Deterministic allocation from consensus-ordered bids
- [x] Task execution with progress tracking
- [x] BFT-replicated reputation system
- [x] Self-healing: abandoned tasks auto-re-auctioned
- [x] 8+ configurable agent types

## 🔧 Technology Stack

- **Rust** — Systems programming with safety guarantees
- **Tashi Vertex** — BFT consensus engine (sub-100ms, leaderless, gasless)
- **Tokio** — Async runtime for concurrent agent operations
- **Serde** — Serialization for wire protocol
- **HTML/CSS/JS** — Real-time monitoring dashboard

## 📊 How It Works

1. **Discovery**: Agents bootstrap with known peer addresses and exchange HELLO messages
2. **Heartbeats**: Every 3 seconds, agents broadcast their status through Vertex
3. **Task Broadcasting**: Any agent can submit a task to the swarm
4. **Sealed-Bid Auction**: Capable agents submit bids; Vertex fair ordering prevents front-running
5. **Deterministic Allocation**: All nodes compute the same winner from consensus-ordered bids
6. **Execution**: Winner executes the task, broadcasting progress updates
7. **Reputation**: Success/failure updates replicated via BFT consensus
8. **Self-Healing**: If an agent goes stale, its tasks are automatically re-auctioned

## 👤 Built By

**0xShobha** — [GitHub](https://github.com/0xshobha)

## 📄 License

Apache 2.0
