# APEX Middleware

> **The intelligent operational bus for connected robots, autonomous systems, and distributed operations.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/runtime-Rust-orange)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/control--plane-TypeScript-blue)](https://www.typescriptlang.org/)
[![Status](https://img.shields.io/badge/status-Phase%201%20Active-brightgreen)]()

---

## What is APEX?

APEX is a **proprietary, distributed, decentralized pub/sub middleware** built from first principles for robotics, edge-cloud operations, and multi-robot coordination.

It is **not** a ROS 2 clone. It is **not** a DDS wrapper. It is **not** a generic message broker.

APEX is an **operational runtime** with its own:
- Binary serialization protocol
- Adaptive peer discovery (no single point of failure)
- QoS engine with operational contracts
- Hybrid data plane: shared memory IPC + UDP + TCP
- Native observability, health, and diagnostics
- Decoupled control plane with RBAC and audit
- Memory governance with bounded resources
- Slow consumer detection and congestion control
- Capability negotiation between heterogeneous nodes
- Graceful degradation under network failure

---

## Tagline

> *"APEX transforms distributed communication, robotics integration, edge-cloud operation and intelligent coordination into a single coherent system."*

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    APEX Runtime                             │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Node A   │  │ Node B   │  │ Node C   │  │ Node D   │  │
│  │ [Robot]  │  │ [Sensor] │  │ [Planner]│  │ [Cloud]  │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       │              │              │              │         │
│  ════════════════ DATA PLANE ═══════════════════════════   │
│  │  SHM (IPC zero-copy)  │  UDP (streaming) │  TCP (ctrl)  │
│  ════════════════════════════════════════════════════════   │
│       │              │              │              │         │
│  ════════════════ CONTROL PLANE ════════════════════════   │
│  │ Discovery │ QoS Engine │ Memory Gov │ Observability │   │
│  ════════════════════════════════════════════════════════   │
└─────────────────────────────────────────────────────────────┘
```

---

## Product Lines

| Product | Description |
|---|---|
| **APEX Core** | Runtime, topics, nodes, serializer, discovery, QoS, health |
| **APEX Connect** | Edge-cloud sync, remote ops, gateway management |
| **APEX Swarm** | Multi-robot, peer coordination, topology-aware mesh |
| **APEX Industrial** | OPC UA, MQTT, Modbus bridges, audit trail |
| **APEX Med** | Critical teleop, auditable replay, safety layers |
| **APEX Lab** | Benchmark suite, fault injection, diagnostics |

---

## Repository Structure

```
apex/
  runtime/               # Core runtime (Rust)
    core/                # Types, errors, identifiers
    node/                # Node lifecycle
    topic/               # Topic registry
    publisher/           # Publisher engine
    subscriber/          # Subscriber engine
    discovery/           # Adaptive peer discovery
    capability-negotiation/
    serializer/          # Binary serialization protocol
    schema-registry/
    compatibility/       # Schema evolution & compat matrix
    qos/                 # QoS engine
    congestion/          # Congestion control & slow consumers
    rate-control/
    shm/                 # Shared memory transport
    udp/                 # UDP transport
    tcp/                 # TCP transport
    memory/              # Memory pool & governance
    buffer/              # Buffer manager
    scheduler/
    health/              # Liveness, readiness, degraded mode
    diagnostics/         # Snapshots, reports
    observability/       # Metrics, tracing, structured logs
    security/            # Identity, session epochs
    session/             # Session lifecycle
    replay/              # Replay hooks
    benchmark/           # Benchmark lab
    fault-injection/     # Fault injection
    config/              # Validated configuration
    shutdown/            # Graceful shutdown orchestrator
    tests/
  sdk/
    cpp/                 # C++20 SDK
    rust/                # Rust SDK (native)
    ts/                  # TypeScript SDK
  control-plane/
    api/                 # NestJS/Fastify backend
    web/                 # Next.js dashboard
  docs/
  specs/                 # Architecture specs
  examples/
  tools/
  scripts/
  infra/
```

---

## Quick Start (Developer API)

```cpp
// C++20 API
auto node = apex::Node::create("sensor_laser", apex::NodeOptions{
    .profile = apex::DeploymentProfile::MobileRobotics,
    .discovery_mode = apex::DiscoveryMode::Adaptive
});

auto publisher = node.create_publisher<LaserScan>(
    "scan",
    apex::Qos{
        .reliability = apex::Reliability::BestEffort,
        .history = apex::History::KeepLast(8),
        .late_join_policy = apex::LateJoinPolicy::Recent,
        .queue_limit = 32
    }
);

auto loan = publisher.loan();
fill_scan(loan.get());
publisher.publish_loaned(std::move(loan));
```

```rust
// Rust native API
let node = apex::Node::builder("sensor_laser")
    .profile(DeploymentProfile::MobileRobotics)
    .discovery(DiscoveryMode::Adaptive)
    .build()?;

let pub = node.create_publisher::<LaserScan>(
    "scan",
    Qos::builder()
        .reliability(Reliability::BestEffort)
        .history(History::KeepLast(8))
        .queue_limit(32)
        .build()
)?;

let mut loan = pub.loan()?;
loan.fill_scan_data(&scan_data);
pub.publish_loaned(loan)?;
```

---

## Design Principles

1. **No DDS** — Discovery, QoS, serialization and transport are all custom-built
2. **Bounded resources** — No infinite queues, no uncontrolled memory growth
3. **Degraded-first** — System stays operational under network failure, slow consumers, and partial discovery
4. **Observability native** — Every subsystem emits structured logs, metrics, and diagnostics
5. **Compatibility explicit** — Schema mismatch, QoS mismatch, and capability gaps are always surfaced
6. **Security by default** — Node identity, session epochs, RBAC, safe defaults
7. **Clean architecture** — Domain without framework, bounded contexts, explicit contracts

---

## Deployment Profiles

| Profile | Use Case |
|---|---|
| `dev-local` | Single machine development |
| `lab-simulation` | Simulated multi-robot lab |
| `office-lan` | Stable LAN operation |
| `factory-floor` | Industrial deployment |
| `mobile-robotics` | Mobile robots, unstable connectivity |
| `edge-cloud` | Edge-cloud hybrid |
| `unstable-wifi` | Wi-Fi first-class support |
| `constrained-embedded` | Resource-limited nodes |
| `high-throughput-sensor` | Sensor-heavy topologies |
| `remote-supervision` | Remote teleop |

---

## Runtime Milestones

- [x] **Phase 1** — Core local: runtime, node lifecycle, topic registry, serializer, SHM pub/sub, benchmark
- [ ] **Phase 2** — Network: UDP/TCP, health, observability, reconnect
- [ ] **Phase 3** — Adaptive discovery: multicast bootstrap, TTL, keepalive, static peer fallback
- [ ] **Phase 4** — QoS + compatibility: reliability, history, late join, schema hash, capability negotiation
- [ ] **Phase 5** — Operational robustness: memory governance, slow consumer, congestion, diagnostics
- [ ] **Phase 6** — Lab: benchmark suite, fault injection, profile simulation, replay hooks
- [ ] **Phase 7** — Control plane: auth, RBAC, dashboard, logs, metrics, alerts, audit
- [ ] **Phase 8** — Expansion: edge-cloud, industrial bridges, AI hooks

---

## Stack

| Layer | Technology |
|---|---|
| Runtime | Rust (no_std future target) |
| C++ SDK | C++20 |
| Control Plane API | TypeScript + NestJS/Fastify |
| Control Plane Web | Next.js + React + TypeScript |
| Database | PostgreSQL + Prisma |
| State Management | Zustand + TanStack Query |
| Auth | JWT (custom) |
| CI | GitHub Actions |
| Infra | Docker + Docker Compose |

---

## License

MIT — see [LICENSE](LICENSE)

---

> APEX is built to survive the real world: bad Wi-Fi, incomplete peer discovery, limited memory, large payloads, slow subscribers, dynamic reconfiguration, multi-robot expansion, edge-cloud integration, and future critical systems.
