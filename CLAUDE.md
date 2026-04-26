# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Architecture Overview

f1-dash is a real-time F1 telemetry dashboard split into four services:

- **`dashboard/`** — Next.js 16 / React 19 frontend (TypeScript, Tailwind v4, Zustand, yarn v4)
- **`realtime/`** — Rust/Axum service that connects to F1's SignalR endpoint and streams data to clients via SSE
- **`api/`** — Rust/Axum service for non-realtime data (schedule, health checks)
- **`signalr/`** — Shared Rust crate that implements the SignalR client protocol used by `realtime`
- **`shared/`** — Shared Rust utilities (tracing setup, JSON merge logic)
- **`simulator/`** — Rust tool for replaying saved `.data.txt` recordings locally

### Data Flow

```
F1 SignalR → [realtime] → SSE /api/realtime → [dashboard useSocket]
                                                      ↓
                                              useDataEngine (buffers + delay)
                                                      ↓
                                              useDataStore (Zustand)
                                                      ↓
                                              React components
```

`realtime` merges incoming F1 topic updates into an in-memory `StateService` (Arc<RwLock<Value>>). The dashboard connects via `EventSource`, receives `initial` and `update` events, buffers them with timestamps to support configurable replay delay, and flushes to Zustand at 200ms intervals.

`CarData.z` and `Position.z` topics are zlib-compressed — the dashboard decompresses them with `pako` via `lib/inflate.ts`.

### Rust workspace

All Rust crates live in the workspace root. Default members are `realtime` and `api`.

## Development Commands

### Dashboard (run from `dashboard/`)

```bash
# Install deps (requires corepack enabled)
corepack enable && corepack install
yarn

# Copy env and start dev server
cp .env.example .env
yarn dev          # http://localhost:3000

# Build / prod
yarn build
yarn start

# Lint & format
yarn lint
yarn prettier     # runs prettier --write src
```

Required env vars (see `.env.example`):
- `NEXT_PUBLIC_LIVE_URL` — URL of the `realtime` service (default `http://localhost:4000`)
- `API_URL` — URL of the `api` service (default `http://localhost:4001`)

### Rust services (run from repo root)

```bash
# Start realtime service (port 4000)
cargo run -p realtime

# Start api service (port 4001)
cargo run -p api

# Start simulator with a recording
cargo run -p simulator -- year-circuit.data.txt
```

Set `F1_DEV_URL=ws://localhost:8000/ws` on the `realtime` service to point it at the simulator instead of F1's live endpoint.

### Docker Compose (all services)

```bash
docker compose up
```

Ports: dashboard → 3000, realtime → 4000, api → 4010.

## Key Files

| File | Purpose |
|---|---|
| `dashboard/src/hooks/useDataEngine.ts` | Core data pipeline: buffers, delay logic, 200ms flush interval |
| `dashboard/src/hooks/useSocket.ts` | SSE connection to realtime service |
| `dashboard/src/stores/useDataStore.ts` | Zustand store: `state`, `carsData`, `positions` |
| `dashboard/src/stores/useSettingsStore.ts` | User settings including `delay` (seconds) |
| `dashboard/src/env.ts` | Zod-validated env schema; client gets vars injected via `EnvScript` |
| `dashboard/src/app/dashboard/layout.tsx` | Wires socket + data engine into layout; handles syncing state |
| `realtime/src/f1.rs` | Subscribes to 17 F1 topics, restarts on `SessionInfo` change |
| `realtime/src/services/state_service.rs` | In-memory state as `Arc<RwLock<Value>>` with recursive merge |
| `shared/src/lib.rs` | `merge()` — recursive JSON merge (object keys and array indices by numeric key) |
| `signalr/src/lib.rs` | SignalR negotiate → WebSocket connect → subscribe → stream |

## Conventions

- Branching: `feature/name` or `bugfix/name` based off `develop`, merged into `develop`.
- Commits: conventional commits (`feat`, `fix`, `refactor`, `perf`, `chore`).
- Before a PR: run `yarn build` (frontend) and verify it starts cleanly; run `yarn prettier` on touched files.
- Telemetry recordings use `.data.txt` extension (gitignored) to avoid accidental commits.
- `ORIGIN` env var on Rust services accepts semicolon-separated origins.
