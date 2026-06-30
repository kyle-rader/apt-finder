# 🏙️ Apartment Finder

A small, locally-run web app for comparing apartments while flat-hunting. Add
apartments to a search list, see them on a map with ½ / 1 / 2-mile rings, measure
**real walking distance** from every apartment to places that matter to you (a park,
a theatre, a bar), and run **AI research** on each apartment for things listings
don't expose — using your own local LLM via the Claude Code CLI.

Built around a concrete use case: *find a studio near Cal Anderson Park and compare
the walk to the Paramount Theatre, The Cuff, etc.*

> Design and the reasoning behind it live in [`ARCHITECTURE.md`](./ARCHITECTURE.md).

## How it works

- **Search list** — add an apartment by address; it's geocoded and pinned on the map.
- **Walking stats** — pick a target location (searched on the map). The app computes
  the foot-routing distance + time from **every** apartment to it and saves it as a
  reusable column. Add a new apartment later and existing stats backfill automatically.
- **AI stats** — write a research question (e.g. *"in-unit laundry? what do reviews say
  about noise?"*). The app runs your local `claude` CLI once per apartment to research
  the building's site and reviews, and stores a short answer (hover a cell for the full
  response).
- Everything is compared side-by-side in a table and visualized on the map.

## Architecture

A single Rust (Axum) binary serves both the JSON API (`/api/*`) and the built Svelte
SPA. It owns a local SQLite database and is the only thing that talks to outside
services, so the browser never needs API keys or hits CORS issues.

```
Browser (Svelte + Leaflet)
        │  fetch /api/*
        ▼
Axum (Rust)  ──►  SQLite
        ├──►  Nominatim   (geocoding / place search)
        ├──►  OSRM        (walking routes)
        └──►  `claude -p` (AI research subprocess)
```

Open-source map stack only — no Google/Mapbox keys:
[Leaflet](https://leafletjs.com) + OpenStreetMap tiles,
[Nominatim](https://nominatim.org) for geocoding,
[OSRM](https://project-osrm.org) for foot routing.

## Prerequisites

- **Rust** (stable) — https://rustup.rs
- **Node.js 18+** — to build the frontend
- **Claude Code CLI** (`claude`) for AI stats — https://claude.com/claude-code
  (logged in; `claude -p "hello"` should work). Optional if you only use walking stats.

## Setup & run

```bash
# 1. Build the frontend (outputs frontend/dist)
cd frontend
npm install
npm run build
cd ..

# 2. Run the backend (serves the API + the built frontend)
cargo run
```

Then open **http://localhost:8080**.

### Development (hot-reload frontend)

Run the backend and the Vite dev server in two terminals:

```bash
cargo run                      # terminal 1 — API on :8080
cd frontend && npm run dev     # terminal 2 — UI on :5173, proxies /api to :8080
```

Open http://localhost:5173.

## Configuration

All optional, via environment variables:

| Variable        | Default                              | Purpose                                            |
| --------------- | ------------------------------------ | -------------------------------------------------- |
| `PORT`          | `8080`                               | HTTP port                                          |
| `DB_PATH`       | `apt_finder.db`                      | SQLite file (created if missing)                   |
| `NOMINATIM_URL` | `https://nominatim.openstreetmap.org`| Geocoding endpoint (self-host for heavy use)       |
| `OSRM_URL`      | `https://router.project-osrm.org`    | Foot-routing endpoint                              |
| `AI_COMMAND`    | `claude`                             | Executable for AI research (Claude Code CLI)       |
| `STATIC_DIR`    | `frontend/dist`                      | Built frontend assets to serve                     |
| `EXTRA_CA_CERT` | _(unset)_                            | Path to an extra PEM CA cert for outbound HTTPS (e.g. behind a TLS-intercepting proxy) |

### AI research details

For each apartment, the app runs:

```
claude -p "<research prompt>" --output-format json --permission-mode bypassPermissions
```

`bypassPermissions` lets Claude use its web tools non-interactively. Runs are
asynchronous with a small concurrency limit; the UI polls and fills cells in as
each finishes. To use a different LLM CLI, point `AI_COMMAND` at it (it must accept
the same `-p / --output-format json` flags, or adapt `src/services/ai.rs`).

## Tech notes

- The public Nominatim/OSRM demo servers have usage limits and are best-effort. For
  heavy use, run your own instances and set `NOMINATIM_URL` / `OSRM_URL`.
- Data lives entirely in the local SQLite file — delete it to start fresh.

## Project layout

```
src/                 Rust backend
  main.rs            router + static serving + startup
  db.rs              SQLite pool + schema
  models.rs          Apartment / Stat / StatValue
  state.rs           shared AppState
  compute.rs         stat fan-out (walking inline, AI background)
  handlers/          apartments, geocode, stats, /api/state
  services/          nominatim, osrm, ai (claude subprocess)
frontend/            Svelte + Vite SPA (Leaflet map, panels, comparison table)
```

## Tests

```bash
cargo test          # backend unit tests
cargo clippy        # lints
```
