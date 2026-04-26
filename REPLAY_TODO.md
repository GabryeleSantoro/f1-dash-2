# Replay Feature — Implementation TODO

## Key insight: no local storage needed

F1 exposes a public static CDN that serves full historical session data on demand:

- `https://livetiming.formula1.com/static/{year}/Index.json`
  → all meetings + sessions for a year, each with a `Path` and numeric `Key`
- `https://livetiming.formula1.com/static/{path}Index.json`
  → all available feeds for a session (`KeyFramePath` + `StreamPath`)
- `https://livetiming.formula1.com/static/{path}{topic}.json`
  → snapshot of final topic state (used as initial state)
- `https://livetiming.formula1.com/static/{path}{topic}.jsonStream`
  → newline-delimited time-series: `HH:MM:SS.mmm{...json...}` per line

No recordings, no simulator, no shared volumes required.

---

## Phase 1 — `api`: archive endpoint

- [x] Add `GET /api/archive` endpoint in `api/src/endpoints/archive.rs`
  - Fetch `livetiming.formula1.com/static/{current_year}/Index.json`
  - Optionally accept `?year=` query param to browse older seasons
  - Cache result with `io_cached` (same pattern as `schedule.rs`, 30 min TTL)
  - Return `Vec<Meeting>` mirroring the F1 index shape:
    ```
    Meeting { key, name, location, country, sessions: Vec<Session> }
    Session { key, name, type, path, start_date, end_date, gmt_offset }
    ```
- [x] Register route `GET /api/archive` in `api/src/main.rs`
- [x] Add `GET /api/archive/session?path=...` that proxies `{path}Index.json`
  from F1 CDN and returns available feeds — lets the frontend know which
  topics are present before starting replay

---

## Phase 2 — `realtime`: replay mode

### 2a — Source abstraction

- [x] Replace the hardcoded `ingest_f1` loop with a `Source` enum:
  ```rust
  enum Source {
      Live,
      Archive { path: String, speed: f32 },
  }
  ```
- [x] Add a `tokio::watch::Sender<Source>` to `AppState` / `Context`
- [x] Refactor `main.rs` ingestion loop to read from the watch channel;
  cancel current task and restart when source changes

### 2b — Archive ingest function

- [x] Create `realtime/src/archive.rs`:
  - `async fn ingest_archive(path, speed, state_service, sender, replay_state, source_rx)`
  - **Initial state**: start empty `{}` (deviated from spec — keyframes are end-of-session
    state, not start; jsonStream first lines (~t=0) carry the true initial topic state)
  - **Replay loop**: fetches all topics in parallel, merges to single timeline sorted
    by `HH:MM:SS.mmm` offset, walks sleeping `dt/speed` between entries
  - Broadcasts `update` per entry (same shape as live)
  - Position via `ReplayState { position_ms, total_ms }` — both `Arc<AtomicU64>`

### 2c — Replay control endpoints

- [x] Add `realtime/src/http_server/replay.rs` with:
  - `POST /api/replay/start` — body `{ path: String, speed: f32 }`
    → set watch channel to `Source::Archive { path, speed }`
  - `POST /api/replay/stop` → set watch channel to `Source::Live`
  - `GET  /api/replay/status` → `{ mode: "live"|"archive", path?, positionMs?, totalMs?, speed? }`
- [x] Register routes in `http_server.rs`
- [x] Add a `"reset"` SSE event type emitted before switching source
  (clients must clear state on receiving it)

---

## Phase 3 — Dashboard: archive page

- [ ] Create `dashboard/src/app/(nav)/archive/page.tsx`
  - Fetch `GET /api/archive` (from `api` service via `API_URL`)
  - Group sessions by meeting, display as an expandable list
  - Each session row: name, date, type badge, **"Watch"** button
  - "Watch" calls `POST realtime/api/replay/start` with the session `path`
    then navigates to `/dashboard`

- [ ] Create `dashboard/src/lib/fetchArchive.ts`
  - `getArchive(year?)` — server-side fetch from `API_URL/api/archive`
  - Types matching the `api` response shape

- [ ] Add "Archive" link to `Sidebar.tsx` navigation

---

## Phase 4 — Dashboard: playback controls

- [ ] Create `dashboard/src/hooks/useReplayStatus.ts`
  - Polls `GET /api/replay/status` (from `NEXT_PUBLIC_LIVE_URL`) every second
  - Returns `{ isReplay, path, positionMs, totalMs, speed }`

- [ ] Create `dashboard/src/hooks/useReplayControls.ts`
  - `stop()` → `POST /api/replay/stop`
  - `setSpeed(n: 0.5 | 1 | 2 | 4)` → `POST /api/replay/start` with same
    path + new speed (restarts from current position — acceptable for now)

- [ ] Create `dashboard/src/components/ReplayBar.tsx`
  - Shown in `DashboardLayout` only when `isReplay === true`
  - Replaces the live `ConnectionStatus` indicator
  - Contents: session name, progress bar (positionMs / totalMs),
    speed selector buttons (0.5× 1× 2× 4×), **Stop** button
  - Progress bar is read-only for now (seeking is a stretch goal)

- [ ] Handle `"reset"` SSE event in `useSocket.ts`
  - On `reset` event: call `dataStore.setState(null)` and
    `dataStore.setCarsData(null)` and `dataStore.setPositions(null)`
  - Clears stale data before the new `initial` event arrives

- [ ] Show "Replay ended" state in `DashboardLayout` when SSE closes
  normally after an archive session finishes (distinct from a
  connection error)

---

## Phase 5 — Wiring & polish

- [ ] Update `compose.yaml`: expose `realtime` replay control endpoints
  (already on port 4000, no change needed — just document)
- [ ] Update `CLAUDE.md` with replay-specific env vars and endpoints
- [ ] Update `SETUP.md` with archive feature description
- [ ] Stretch: seek support — `POST /api/replay/seek { position_ms }`,
  re-fetch all keyframes up to that point and restart stream from
  nearest `.jsonStream` line

---

## Data flow summary (replay mode)

```
User clicks "Watch" on archive page
  → POST realtime/api/replay/start { path, speed }
     → watch channel → Source::Archive
     → cancel live ingest task
     → emit "reset" SSE → dashboard clears state
     → fetch all {topic}.json keyframes
     → merge → state_service.set_state()
     → emit "initial" SSE → dashboard populates
     → stream .jsonStream lines sorted by timestamp
     → emit "update" SSE at wall-clock-adjusted intervals
  → dashboard shows ReplayBar with progress
User clicks Stop
  → POST realtime/api/replay/stop
     → watch channel → Source::Live
     → emit "reset" SSE
     → reconnect to F1 SignalR
```
