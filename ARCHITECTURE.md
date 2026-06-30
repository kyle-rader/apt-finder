# Architecture

This document describes how Apartment Finder is built and **why** the key
decisions were made. It's meant to be reviewed and revised — each decision lists
the rationale and the main alternative, so you can see what would change if you
picked differently. The `README.md` covers setup/usage; this file covers design.

> Status: initial version (v0.1). Single-user, local-first tool.

---

## 1. System overview

Apartment Finder is a **local-first web app**: one Rust binary serves a JSON API
**and** the built web UI, owns a local SQLite database, and is the only component
that talks to outside services. You run it on your own machine; nothing is hosted.

```
                       ┌─────────────────────────────────────────┐
  Browser              │  Rust binary (Axum)                      │
  ┌───────────────┐    │                                          │
  │ Svelte SPA    │    │  /api/*  ──►  handlers ──► compute ──┐    │
  │  + Leaflet    │◄──►│                                     │    │
  │               │    │  /        ──►  static SPA (dist/)    │    │
  └───────────────┘    │                                     ▼    │
                       │                              ┌──────────┐ │
                       │                              │ SQLite   │ │
                       │                              └──────────┘ │
                       │   outbound (server-side only):            │
                       │     ├─► Nominatim   (geocoding)           │
                       │     ├─► OSRM        (walking routes)      │
                       │     └─► `claude -p` (AI research, subproc)│
                       └─────────────────────────────────────────┘
```

**Core idea — the data model drives the UX.** Three tables (`apartments`,
`stats`, `stat_values`) make the central feature — *"save this metric for every
apartment in my list"* — a simple fan-out, and let a newly-added apartment
backfill all existing metrics automatically.

---

## 2. Key decisions

### D1. Local-first single binary (server owns all I/O)
**What:** One Axum process serves the API + the SPA + owns SQLite, and is the
only thing that calls Nominatim/OSRM/the LLM. The browser never calls third
parties directly.
**Why:** No API keys in the browser, no CORS wrangling, one place to add caching
or rate-limiting later, and trivial deployment (`cargo run`). Fits a personal
tool you run locally.
**Alternative:** A static frontend calling third-party APIs directly — simpler to
host, but leaks keys to the client, hits CORS limits, and can't centralize
caching. Rejected for a key-free, local tool.

### D2. Open-source map stack (OSM / Nominatim / OSRM), no Google
**What:** Leaflet + OpenStreetMap tiles for display, Nominatim for geocoding,
OSRM (foot profile) for walking routes. Endpoints are env-configurable.
**Why:** Zero API keys / billing — "clone and run." OSRM gives real
street-network walking distances, not straight-line guesses.
**Trade-off:** The public demo servers (`nominatim.openstreetmap.org`,
`router.project-osrm.org`) have usage limits and are best-effort. For heavy use,
self-host and set `NOMINATIM_URL` / `OSRM_URL`.
**Alternative:** Google Maps (Distance Matrix) — higher-quality data but requires
a billed API key. Left as a future pluggable provider.

### D3. Radius rings are straight-line circles; real routing is per-target
**What:** The ½ / 1 / 2 mi rings drawn around each apartment are simple circles.
Actual walking distance is computed only for **walking stats** (apartment → a
chosen target) via OSRM.
**Why:** Circles are instant and free (no API calls) and give a good at-a-glance
sense of proximity; precise walking cost is what you actually compare, and that's
done per saved target.
**Alternative:** True walking **isochrones** (street-network reachability blobs)
for the rings — more accurate but needs an isochrone service and is materially
more complex. Deferred.

### D4. Stats as a first-class, fan-out concept
**What:** A `stat` is a saved metric definition that applies to the whole list,
of `kind` `"walking"` or `"ai"`. Creating one computes a `stat_value` for every
apartment; adding an apartment backfills every existing stat.
**Why:** This is the heart of the product — compare *all* candidates on the *same*
yardsticks. Modeling it once (not per-apartment ad hoc) keeps the comparison
table and backfilling trivial.
**See:** `src/compute.rs` (`run_stat_for_all`, `run_all_stats_for_apartment`).

### D5. Walking stats computed inline; AI stats run in the background
**What:** Walking values are computed synchronously during the request (OSRM is
fast). AI values are written as `pending`, then a background task processes them
with bounded concurrency (3 at a time); the UI polls and fills cells in.
**Why:** Each AI call spawns a heavy LLM subprocess that can take many seconds —
blocking the HTTP request (or running 5 at once) would be a bad experience. The
concurrency cap protects the machine and any rate limits.
**See:** `src/compute.rs` (`spawn_ai_run`, `AI_CONCURRENCY`), polling in
`frontend/src/lib/store.svelte.js` (`startPolling`).

### D6. AI research = shell out to the Claude Code CLI
**What:** AI stats run `claude -p "<prompt>" --output-format json
--permission-mode bypassPermissions` per apartment (`AI_COMMAND` is
configurable). The prompt asks the model to research the building's site/reviews
and end with a single `ANSWER:` line, which we parse into the cell value; the
full response is kept for the detail view.
**Why:** Reuses the user's existing Claude Code login and the CLI's built-in web
tools — no API key to manage, no separate web-search wiring.
`bypassPermissions` is required so tools run without an interactive prompt.
**Trade-off / risk:** Tightly coupled to the Claude CLI's flags and JSON shape;
a different CLI needs `AI_COMMAND` plus possibly edits to `src/services/ai.rs`.
We parse defensively (fall back to raw stdout / last non-empty line).

### D7. Rust + Axum + SQLite (sqlx) backend
**What:** Axum 0.8 on Tokio; SQLite via `sqlx` using the **runtime query API**
(`query_as`), not the compile-time-checked macros; schema applied on startup via
`sqlx::raw_sql` (idempotent `IF NOT EXISTS`).
**Why:** A single static binary, strong typing, easy async (subprocess + HTTP).
Runtime queries avoid needing a live database at compile time. SQLite means
zero-setup local persistence in one file.
**Alternative:** sqlx compile-time macros (needs `DATABASE_URL` at build) or a
heavier migration tool — unnecessary for a single-version local app.

### D8. Svelte 5 + Vite SPA frontend (not SvelteKit)
**What:** A plain Svelte 5 (runes) SPA built by Vite to static files, served by
Axum. Leaflet for the map. Dev uses Vite's proxy `/api → :8080`.
**Why:** Keeps deployment to one Rust server + a static `dist/`. SvelteKit brings
its own Node server/SSR, which would split the backend story.
**Alternative:** SvelteKit, or a React SPA — both viable; Svelte+Vite is the
lightest fit for "one binary serves everything."

### D9. Frontend polls a single `/api/state` snapshot
**What:** The UI loads and refreshes via one endpoint returning
`{apartments, stats, values}`. It polls fast (~2.5s) while anything is `pending`,
slowly (~15s) when idle.
**Why:** Dead-simple state sync and exactly what async AI results need (watch
cells fill in). One round-trip keeps the client logic small.
**Alternative:** WebSocket/SSE push — lower latency and no idle polling, but more
moving parts than this scale warrants. Easy to add later if needed.

### D10. Single implicit search list (for now)
**What:** There's one global list of apartments; no "lists" table yet.
**Why:** Matches the immediate use case and keeps v1 lean.
**Growth path:** Add a `lists` table and a `list_id` FK on `apartments`; the
stats/values model is unaffected.

---

## 3. Data model

SQLite, defined in `src/db.rs`. Cascade deletes are on (`foreign_keys` pragma),
so removing an apartment or stat cleans up its values.

| Table          | Purpose | Notable columns |
| -------------- | ------- | --------------- |
| `apartments`   | The search list | `name`, `address`, `lat`, `lng`, `source_url`, `notes` |
| `stats`        | A metric definition for the whole list | `kind` (`walking`\|`ai`); walking: `target_label/lat/lng`; ai: `prompt` |
| `stat_values`  | One computed cell per (stat, apartment) | `value_text`, `value_number`, `status` (`pending`\|`ok`\|`error`), `detail`; `UNIQUE(stat_id, apartment_id)` |

`stat_values` is upserted on the unique pair, so recompute/backfill is idempotent
(`src/compute.rs::upsert_value`). `detail` holds the OSRM summary JSON or the full
LLM response (surfaced on hover in the table).

---

## 4. Backend API

All under `/api` (`src/main.rs::build_router`). Handlers in `src/handlers/`.

| Method & path | Handler | Notes |
| ------------- | ------- | ----- |
| `GET /api/state` | `handlers::full_state` | apartments + stats + values in one payload (load/poll) |
| `GET /api/apartments` | `apartments::list` | |
| `POST /api/apartments` | `apartments::create` | geocodes the address, stores it, backfills all stats |
| `DELETE /api/apartments/{id}` | `apartments::delete` | values cascade |
| `GET /api/geocode?q=` | `geocode::geocode` | Nominatim proxy for the search boxes |
| `GET /api/stats` | `stats::list` | |
| `POST /api/stats` | `stats::create` | validates by kind, then fans out across the list |
| `POST /api/stats/{id}/recompute` | `stats::recompute` | re-run for all apartments |
| `DELETE /api/stats/{id}` | `stats::delete` | values cascade |

Errors funnel through `src/error.rs::AppError` (any `anyhow::Error` → JSON;
chosen status for validation/not-found), so handlers use `?` freely.

---

## 5. Module map

```
src/
  main.rs        Axum router, static SPA serving (ServeDir + index fallback),
                 reqwest client (User-Agent + optional EXTRA_CA_CERT), startup
  config.rs      Config::from_env — PORT, DB_PATH, NOMINATIM_URL, OSRM_URL,
                 AI_COMMAND, STATIC_DIR, EXTRA_CA_CERT
  state.rs       AppState { db pool, http client, config } shared with handlers
  db.rs          SQLite pool + schema (raw_sql, idempotent)
  models.rs      Apartment / Stat / StatValue (+ request bodies)
  error.rs       AppError -> JSON responses
  compute.rs     fan-out: compute_walking (inline), compute_ai (bg), upsert,
                 prompt building, bounded-concurrency AI runner
  handlers/      apartments.rs, geocode.rs, stats.rs, mod.rs (full_state)
  services/      nominatim.rs (geocode), osrm.rs (walk), ai.rs (claude subproc)

frontend/src/
  App.svelte                 layout, rings toggle, selected stat, poll lifecycle
  lib/store.svelte.js        shared reactive state, value index, poll loop, colors
  lib/api.js                 fetch wrapper for /api/*
  lib/Map.svelte             Leaflet: markers, ½/1/2 mi rings, target + connectors
  lib/ApartmentPanel.svelte  add/remove apartments
  lib/StatsPanel.svelte      create walking/AI stats, recompute, "show on map"
  lib/GeoSearch.svelte       reusable Nominatim place search
  lib/ComparisonTable.svelte apartments × stats grid (pending/error/value states)
```

---

## 6. Configuration

Everything is env-driven with zero-setup defaults (see `src/config.rs` and the
README's config table). Notably `EXTRA_CA_CERT` lets the outbound HTTPS client
trust an extra CA — useful behind a TLS-intercepting proxy.

---

## 7. Known limitations & natural next steps

- **Map rings are straight-line**, not walking isochrones (D3).
- **No caching** of geocode/route results yet — each call hits the upstream
  service. The single-owner design (D1) makes adding a cache localized.
- **AI integration is Claude-CLI-shaped** (D6); other CLIs need adaptation.
- **One search list** (D10); multiple named lists is the obvious extension.
- **Comparison table isn't sortable** yet, and AI `value_number` is unused
  (AI answers are text-only) — sorting/ranking is a likely enhancement.
- **No auth / multi-user** — by design, this is a local single-user tool.
