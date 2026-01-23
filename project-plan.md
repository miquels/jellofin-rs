# jellofin-rs project plan

## Scope / goals

- Implement a Jellyfin-compatible server in Rust by porting the Go reference implementation found in `jellofin-server/`.
- Implement **both APIs**:
  - Jellyfin API (subset implemented by Go server; focus on library/media endpoints; no transcoding)
  - Notflix API (`/api/*` JSON + `/data/*` file server + optional image resizing)
- Keep **YAML configuration format compatible** with Go `jellofin-server.yaml`.
- Use the planned tech stack:
  - `axum` + `tokio`
  - `sqlx` (sqlite)
  - `image` for image operations
  - `tantivy` for indexing/search
  - `clap` for CLI parsing

## Non-goals

- Transcoding (explicitly not supported in the Go server).
- Full Jellyfin API parity (only what the Go server implements + what clients require).

## Target architecture

### Crate layout

- `src/lib.rs`: library crate containing all functionality.
- `src/bin/main.rs`: thin binary that parses CLI args, loads config, and calls `jellofin_rs::run(...)`.

### High-level modules (Rust)

- `config`: YAML schema + CLI options
- `server`: build axum router; start HTTP(S) server
- `middleware`: request normalization, request logging, compression
- `db`:
  - `repo` traits mirroring Go `database.Repository`
  - `sqlite` implementation using `sqlx`
  - migrations (schema)
  - background tasks (periodic flush of caches)
- `collection`:
  - scanner (movies/shows)
  - in-memory model (Movie/Show/Season/Episode)
  - metadata (NFO + filename fallback)
  - search index (tantivy)
- `imageresize`: open/resize/cache
- `notflix`: handlers + types
- `jellyfin`: handlers + types
- `util`: id hashing, etag helpers, path helpers

### Concurrency model

- Run the HTTP server on `tokio`.
- Use `Arc<...>` state shared via axum state.
- Background tasks:
  - Collection rescans and index rebuild (easy initial implementation = full rebuild).
  - DB flush loops (access token + userdata) via `tokio::time::interval`.

## Dependencies (initial suggestion)

Keep as close as possible to the mandated stack; add only what is needed:

- Web/server:
  - `axum`
  - `tokio` (full)
  - `tower` / `tower-http` (trace, compression, serve-dir)
- Config/CLI:
  - `clap` (derive)
  - `serde`, `serde_yaml`
- JSON:
  - `serde_json`
- DB:
  - `sqlx` with `sqlite`, `runtime-tokio`, `macros`, `chrono` (if needed)
- Search:
  - `tantivy`
- Images:
  - `image`
- Misc:
  - `thiserror` (error types)
  - `tracing`, `tracing-subscriber`

## Milestones

### Milestone 0 — Repository scaffolding and CI sanity

**Outcome:** `cargo test` and `cargo fmt` succeed; project has a clean module structure.

- Add `src/bin/main.rs` with clap options:
  - `--config <path>` defaulting to `jellofin-server.yaml`
- Add a top-level `run(config_path)` function in the library.
- Decide on a single app state struct (e.g. `AppState`) that holds:
  - parsed config
  - db repo
  - collection repo
  - imageresizer

### Milestone 1 — Config compatibility (YAML)

**Outcome:** Rust can load the same YAML file as Go (including legacy `dbdir`).

- Implement `config` types mirroring Go `configFile`:
  - `listen.address`, `listen.port`, `listen.tlscert`, `listen.tlskey`
  - `appdir`, `cachedir`, `dbdir`
  - `database.sqlite.filename`
  - `logfile`
  - `collections[]`: `id`, `name`, `type`, `directory`, `baseurl`, `hlsserver`
  - `jellyfin`: `serverId`, `servername`, `autoregister`, `imagequalityposter`
- Ensure defaults match Go:
  - port default `8096`
  - logfile default (Go uses `/dev/stdout` but also supports `stdout` / `none`)

### Milestone 2 — HTTP server skeleton + middleware

**Outcome:** Server starts, normalizes paths, serves a couple endpoints.

- Build axum router with:
  - request path normalization:
    - collapse `//`
    - strip `/emby` prefix
  - request logging
  - (optional) gzip compression
- Implement minimal endpoints:
  - `GET /health` (Jellyfin)
  - `GET /System/Ping` (Jellyfin)
  - `GET /robots.txt`
  - static file serving from `appdir` (root fallback)

### Milestone 3 — Database (sqlx + sqlite schema + basic operations)

**Outcome:** DB schema created via migrations; minimal user/token operations work.

- Create migrations matching Go schema:
  - `items`, `users`, `accesstokens`, `playstate`, `playlist`, `playlist_item`
- Implement `Repo` interface in Rust:
  - users: get by username/id, upsert
  - access tokens: get token, list by user, upsert, delete
  - userdata: get/update favorites/recently-watched
  - playlists: create/get/add/remove/move
- Implement DB background tasks:
  - access token cache flush interval (match Go default ~10s)
  - userdata cache flush interval (match Go default ~10s)

### Milestone 4 — Collection scanning + in-memory model

**Outcome:** Server can scan configured collections and build in-memory items similar to Go.

- Port scanner behavior from `collection/kodifs.go`:
  - movies dir scan (per-folder movie)
  - shows dir scan (season subdirs `Sxx` / specials)
  - episode filename parsing patterns (`s01e02`, `3x08`, date-based)
  - image conventions (`poster`, `fanart`, `banner`, season posters)
  - subtitles: `.srt` and `.vtt` discovery; srt→vtt “expected mapping”
- Port metadata:
  - NFO parsing
  - filename fallback metadata
- Persist minimal item rows to DB (mirroring `DbLoadItem` behavior).

### Milestone 5 — Tantivy search (easy initial implementation)

**Outcome:** Searching works; simplest approach used.

- Implement search index build as a **full rebuild**:
  - Build a list of documents from in-memory items
  - Recreate index on rebuild (in-memory or on-disk path; choose simplest)
- Provide functions used by Jellyfin handlers:
  - `search(term) -> Vec<ItemId>`
  - `similar(item) -> Vec<ItemId>`

### Milestone 6 — Image resize + caching

**Outcome:** `/data/*` and Jellyfin images can return resized posters.

- Implement `imageresize`:
  - support query params `w/h/mw/mh/q`
  - implement basic cache naming strategy (safe and stable)
  - ensure correct content-type for `jpg/png`

### Milestone 7 — Notflix API parity

**Outcome:** Notflix UI / consumers can browse and fetch media assets.

- Implement `/api`:
  - `GET /api/collections`
  - `GET /api/collection/:coll`
  - `GET /api/collection/:coll/genres`
  - `GET /api/collection/:coll/items`
  - `GET /api/collection/:coll/item/:item`
- Implement `/data/:source/*path`:
  - safe path join + traversal prevention
  - ETag and cache-control headers
  - image resize integration
- Implement `/v/*` serving `index.html` from `appdir`.

### Milestone 8 — Jellyfin API (incremental, client-focused)

**Outcome:** Real Jellyfin clients connect and can browse/play.

Implement in the order that reduces integration pain:

1. **Auth + tokens**
   - `POST /Users/AuthenticateByName`
   - token parsing from headers/query:
     - `Authorization` / `X-Emby-Authorization`
     - `X-Emby-Token`, `X-MediaBrowser-Token`
     - `ApiKey` / legacy `api_key`
   - optional `autoregister`

2. **System endpoints**
   - `GET /System/Info`
   - `GET /System/Info/Public`
   - `GET /Plugins`
   - `GET /DisplayPreferences/usersettings`

3. **Library browsing**
   - Root and collection browsing endpoints used by clients:
     - `/Users/Me`, `/Users`, `/UserViews` and legacy `/Users/{user}/Views`
     - `/Items`, `/Items/{id}`, `/Items/Counts`, `/Items/Latest`
     - `/Genres`, `/Studios`

4. **Images**
   - `/Items/{item}/Images/{type}[/{index}]`
   - include redirect tags behavior if needed.

5. **Playback**
   - `/Items/{item}/PlaybackInfo`
   - `/Videos/{item}/stream[.{container}]`

6. **User data**
   - resume:
     - `/UserItems/Resume`
   - played/favorite endpoints

7. **Search + similar + next up**
   - `/Search/Hints`
   - `/Items/{item}/Similar`
   - `/Shows/NextUp`

8. **Playlists**
   - `/Playlists*` endpoints used by the Go server

### Milestone 9 — Integration testing + client smoke tests

**Outcome:** Confidence that behavior matches Go for core flows.

- Add a small set of HTTP integration tests for:
  - config parsing
  - a couple Jellyfin endpoints returning expected shape
  - Notflix collections/items responses
- Run a real client (Infuse / Streamyfin) against the Rust server.

## Implementation guidelines / invariants to keep

- Preserve request-path normalization:
  - collapse `//`
  - strip `/emby` prefix
- Preserve query normalization for Jellyfin (lowercase first character of query param names) if clients rely on it.
- For initial indexing, keep it simple: rebuild from scratch periodically.
- Keep types/JSON fields as close to Go responses as possible.

## Deliverable definition ("done")

- Rust server starts from the same YAML config.
- Notflix UI and basic Notflix API routes function.
- At least one tested Jellyfin client can:
  - authenticate
  - browse libraries
  - fetch images
  - stream a file

