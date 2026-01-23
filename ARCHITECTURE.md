# jellofin-rs Architecture

## Overview

`jellofin-rs` is a Jellyfin-compatible media server written in Rust. It implements both the Jellyfin API (for compatibility with Jellyfin clients) and a custom Notflix API. The server scans media collections, indexes content with full-text search, serves images with on-demand resizing, and streams video files.

**Tech Stack:**
- `axum` + `tokio` - Async HTTP server
- `sqlx` - SQLite database access
- `tantivy` - Full-text search indexing
- `image` - Image processing and resizing
- `serde` + `serde_yaml` - Configuration and JSON serialization

## Project Structure

```
src/
├── lib.rs              # Library entry point, main run() function
├── bin/main.rs         # Binary entry point, CLI argument parsing
├── config/             # YAML configuration loading
├── server/             # HTTP server and routing
├── middleware/         # Request normalization and logging
├── db/                 # Database layer (SQLite)
├── collection/         # Media scanning and in-memory models
├── imageresize/        # Image processing with caching
├── notflix/            # Notflix API handlers
└── jellyfin/           # Jellyfin API handlers
```

## Module Details

### 1. `config` Module

**Purpose:** Load and parse YAML configuration files compatible with the Go jellofin-server.

**Key Types:**
- `Config` - Root configuration structure
- `ListenConfig` - HTTP server settings (address, port, TLS)
- `JellyfinConfig` - Jellyfin-specific settings (server name, ID, autoregister)
- `CollectionConfig` - Media collection definitions

**Configuration Fields:**
- `listen.address` / `listen.port` - Server bind address
- `listen.tlscert` / `listen.tlskey` - Optional TLS certificates
- `dbdir` - SQLite database directory (legacy support)
- `dbpath` - Direct database file path
- `appdir` - Static file serving directory
- `jellyfin.server_name` - Server display name
- `jellyfin.server_id` - Unique server identifier
- `jellyfin.autoregister` - Auto-create users on first login
- `collections[]` - Array of media collections with:
  - `id`, `name`, `type` (movies/shows)
  - `directory` - Root path to scan
  - `base_url`, `hls_server` - Optional streaming URLs

**Usage:** Loaded at startup via `Config::from_file(path)`.

---

### 2. `server` Module

**Purpose:** Build the HTTP router and start the axum server.

**Key Components:**
- `AppState` - Shared application state (Arc-wrapped):
  - `config: Arc<Config>`
  - `db: Arc<SqliteRepository>`
  - `collections: CollectionRepo`
  - `image_resizer: ImageResizer`
- `build_router(state)` - Constructs the axum Router with all routes
- `start_server(config, state)` - Binds and runs the HTTP server

**Middleware Stack (applied in order):**
1. `normalize_path` - Collapse `//`, strip `/emby` prefix
2. `log_request` - Log HTTP method, path, status
3. `CompressionLayer` - Gzip/Brotli compression
4. `TraceLayer` - Distributed tracing

**Static File Serving:**
- If `appdir` is configured, serves files via `ServeDir` fallback

---

### 3. `middleware` Module

**Purpose:** HTTP request preprocessing.

**Functions:**
- `normalize_path()` - Removes duplicate slashes, strips `/emby` prefix for Jellyfin compatibility
- `log_request()` - Logs request method, path, and response status

---

### 4. `db` Module

**Purpose:** SQLite database abstraction with async support.

**Submodules:**
- `model.rs` - Database model structs:
  - `User` - User accounts (id, username, password)
  - `AccessToken` - Session tokens (token, user_id, device info, date_created)
  - `Item` - Media items (id, name, type, metadata)
  - `UserData` - User-specific item data (played, favorite, playback position)
- `repo.rs` - Repository trait definitions:
  - `UserRepo` - User CRUD operations
  - `AccessTokenRepo` - Token management
  - `ItemRepo` - Item storage and retrieval
  - `UserDataRepo` - User data tracking
- `sqlite.rs` - SQLite implementation:
  - `SqliteRepository` - Implements all repo traits
  - In-memory caching for access tokens and user data
  - Periodic flush to disk via background tasks
  - Schema migrations on startup

**Database Schema:**
- `users` - User accounts
- `access_tokens` - Authentication tokens
- `items` - Media item metadata
- `user_data` - Playback state, favorites, etc.

**Caching Strategy:**
- Access tokens and user data kept in-memory (`HashMap`)
- Periodic flush every 30 seconds
- Reduces disk I/O for frequently accessed data

---

### 5. `collection` Module

**Purpose:** Media library scanning, metadata extraction, and in-memory storage.

**Submodules:**

#### `collection.rs`
- `Collection` - In-memory representation of a media library
  - `movies: HashMap<String, Movie>` - Movie items by ID
  - `shows: HashMap<String, Show>` - TV show items by ID
- `CollectionType` - Enum: Movies or Shows
- Methods: `get_item()`, `get_genres()`, `item_count()`

#### `item.rs`
- `Movie` - Movie metadata (name, year, rating, genres, studios, people, images, media sources)
- `Show` - TV show metadata with `seasons: HashMap<i32, Season>`
- `Season` - Season metadata with `episodes: HashMap<i32, Episode>`
- `Episode` - Episode metadata (season/episode numbers, runtime, images, media sources)
- `Person` - Cast/crew information (name, type, role)
- `PersonType` - Enum: Actor, Director, Writer, Producer
- `ImageInfo` - Image file paths (primary, backdrop, logo, thumb, banner)
- `MediaSource` - Video file info (path, size, subtitles)
- `Subtitle` - Subtitle file (path, language, codec)

#### `scanner.rs`
- `scan_collection()` - Main entry point for scanning
- **Movie Scanning:**
  - One movie per directory
  - Looks for video files (mkv, mp4, avi, etc.)
  - Finds images: `poster.jpg`, `fanart.jpg`, `logo.png`, etc.
  - Parses `movie.nfo` for metadata
- **TV Show Scanning:**
  - Show directory → Season subdirs (`Season 01`, `S01`, etc.)
  - Episode filename parsing (regex patterns):
    - `s01e02`, `1x02`, `2024-01-15` (date-based)
  - Season images: `season01-poster.jpg`, `season-all-poster.jpg`
  - Parses `tvshow.nfo` and episode `.nfo` files
- **Subtitle Discovery:**
  - Finds `.srt` and `.vtt` files
  - Matches by filename proximity
  - Language detection from filename

#### `nfo.rs`
- `parse_movie_nfo()` - Extract metadata from movie.nfo XML
- `parse_tvshow_nfo()` - Extract metadata from tvshow.nfo XML
- `parse_episode_nfo()` - Extract metadata from episode .nfo XML
- Parses: title, plot, year, rating, genres, studios, actors, directors

#### `parse_filename.rs`
- `parse_episode_filename()` - Extract season/episode numbers
- Regex patterns for common formats:
  - `s(\d+)e(\d+)` - s01e02
  - `(\d+)x(\d+)` - 1x02
  - Date-based: `(\d{4})-(\d{2})-(\d{2})`

#### `repo.rs`
- `CollectionRepo` - Manages all collections
  - `collections: HashMap<String, Collection>`
  - `search_index: SearchIndex` - Tantivy index
- Methods:
  - `scan_all()` - Scan all configured collections
  - `search(query, limit)` - Full-text search
  - `find_similar(item_id, limit)` - Genre-based similarity
  - `list_collections()`, `get_collection(id)`

#### `search.rs`
- `SearchIndex` - Tantivy-based full-text search
- **Indexed Fields:**
  - `id`, `collection_id`, `item_type`
  - `name`, `overview` (full-text)
  - `genres` (multi-valued)
- **Methods:**
  - `rebuild(collections)` - Full index rebuild
  - `search(query, limit)` - Query parser search
  - `find_similar(item_id, limit)` - Genre-based fuzzy matching

---

### 6. `imageresize` Module

**Purpose:** On-demand image resizing with disk caching.

**Key Type:**
- `ImageResizer` - Manages image processing
  - `cache_dir: PathBuf` - Cache directory (default: `./cache/images`)

**Methods:**
- `resize_image(path, width, height, quality)` - Resize and cache
  - Cache key: SHA256(path + mtime + dimensions + quality)
  - Supports: JPEG (with quality), PNG, WebP, GIF
  - Uses Lanczos3 filter for high-quality downscaling
  - Maintains aspect ratio if only one dimension specified
- `clear_cache()` - Delete all cached images
- `get_cache_size()` - Calculate total cache size

**Caching Strategy:**
- Cache hit: Return cached file immediately
- Cache miss: Load, resize, encode, save to cache
- Cache invalidation: File modification time in cache key

**Supported Formats:**
- Input: JPEG, PNG, WebP, GIF
- Output: Same as input (preserves format)
- Quality parameter: JPEG only (1-100, default 90)

---

### 7. `notflix` Module

**Purpose:** Custom Notflix API for media browsing.

**Submodules:**

#### `types.rs`
- `CollectionInfo` - Collection summary
- `CollectionDetail` - Collection with all items
- `ItemSummary` - Brief item info for listings
- `MovieDetail` / `ShowDetail` - Full item details
- `SeasonInfo` / `EpisodeInfo` - TV show structure
- `GenreCount` - Genre statistics

#### `handlers.rs`
- `list_collections()` - GET `/api/collections`
- `get_collection(id)` - GET `/api/collection/:id`
- `get_collection_genres(id)` - GET `/api/collection/:id/genres`
- `get_collection_items(id, ?genre)` - GET `/api/collection/:id/items`
- `get_item(coll_id, item_id)` - GET `/api/collection/:coll_id/item/:item_id`
- `serve_data_file(source, path, ?width, ?height, ?quality)` - GET `/data/:source/*path`
  - Serves media files and images
  - Integrates with `ImageResizer` for on-demand resizing
  - Path traversal protection

---

### 8. `jellyfin` Module

**Purpose:** Jellyfin API compatibility for standard clients.

**Submodules:**

#### `types.rs`
- `AuthenticationRequest` / `AuthenticationResult` - Login flow
- `UserDto` / `UserPolicy` - User information
- `SystemInfo` / `PublicSystemInfo` - Server metadata
- `BaseItemDto` - Universal item representation (movies, shows, seasons, episodes)
- `MediaSourceInfo` / `MediaStream` - Playback info
- `QueryResult<T>` - Paginated responses
- `SearchHint` - Search results
- `ItemCounts` - Library statistics

**Field Naming:** PascalCase (Jellyfin convention) via `#[serde(rename_all = "PascalCase")]`

#### `auth.rs`
- `authenticate_by_name()` - POST `/Users/AuthenticateByName`
  - Auto-registration if enabled
  - Generates session token (UUID)
  - Returns `AuthenticationResult` with user and token
- `auth_middleware()` - Token extraction middleware
  - Checks multiple sources:
    - `Authorization` header (Emby format: `Token=...`)
    - `X-Emby-Authorization` header
    - `X-Emby-Token` / `X-MediaBrowser-Token` headers
    - `ApiKey` / `api_key` query parameters
  - Injects `user_id` into request extensions
- `get_user_id()` - Extract user ID from request

#### `handlers.rs`

**System Endpoints:**
- `system_info()` - GET `/System/Info`
- `public_system_info()` - GET `/System/Info/Public`
- `plugins()` - GET `/Plugins` (returns empty array)
- `display_preferences()` - GET `/DisplayPreferences/usersettings`

**User Endpoints:**
- `get_users()` - GET `/Users`
- `get_current_user()` - GET `/Users/Me`
- `get_user_views()` - GET `/UserViews` and `/Users/:user_id/Views`

**Library Endpoints:**
- `get_items(?ParentId, ?Limit)` - GET `/Items`
- `get_item_by_id(id)` - GET `/Items/:id`
- `get_latest_items(?ParentId, ?Limit)` - GET `/Items/Latest`
- `get_item_counts()` - GET `/Items/Counts`

**Playback Endpoints:**
- `get_playback_info(id)` - POST `/Items/:id/PlaybackInfo`
- `stream_video(id)` - GET `/Videos/:id/stream[.mkv]`
  - Direct file streaming (no transcoding)
  - Uses tokio async file I/O

**Search Endpoints:**
- `search_hints(?SearchTerm, ?Limit)` - GET `/Search/Hints`
- `get_similar_items(id, ?Limit)` - GET `/Items/:id/Similar`

**Item Conversion:**
- `convert_movie_to_dto()` - Movie → BaseItemDto (public)
- `convert_show_to_dto()` - Show → BaseItemDto (public)
- `convert_season_to_dto()` - Season → BaseItemDto (public)
- `convert_episode_to_dto()` - Episode → BaseItemDto (public)

#### `streaming.rs`

**Video Streaming with HTTP Range Support:**
- `stream_video_with_range(item_id, headers)` - GET `/Videos/:id/stream[.mkv]`
  - Supports HTTP Range requests for seeking and partial content delivery
  - Returns 206 Partial Content for range requests
  - Returns 200 OK for full file requests
  - Includes `Accept-Ranges: bytes` header
  - Handles byte range parsing (e.g., `Range: bytes=0-1023`)
  - Efficient seeking using tokio async I/O
  - Proper Content-Range and Content-Length headers

**Subtitle Streaming:**
- `stream_subtitle(item_id, index)` - GET `/Videos/:id/Subtitles/:index/Stream`
  - Serves subtitle files directly from media sources
  - Supports both SRT and VTT formats
  - Automatic content-type detection
  - Index-based subtitle selection (0-based)
  - Also available at `/Videos/:id/:index/Subtitles`

**Helper Functions:**
- `find_video_file()` - Locates video file and retrieves file size
- `find_subtitle_file()` - Locates subtitle file by index
- `stream_with_range()` - Handles partial content delivery
- `stream_full_file()` - Handles full file streaming

#### `userdata.rs`

**User Data Endpoints:**
- `get_resume_items(?Limit)` - GET `/Users/:user_id/Items/Resume`
  - Returns items with playback position > 0 and not fully played
  - Sorted by most recent playback position
  - Includes both movies and episodes
- `get_next_up(?Limit)` - GET `/Shows/NextUp`
  - Returns next unwatched episode for each show
  - Based on last watched episode per show
  - Automatically advances to next season if needed
- `mark_played(item_id)` - POST `/Users/:user_id/PlayedItems/:id`
  - Marks item as played
  - Increments play count
  - Clears playback position
- `mark_unplayed(item_id)` - DELETE `/Users/:user_id/PlayedItems/:id`
  - Marks item as unplayed
  - Clears playback position
- `mark_favorite(item_id)` - POST `/Users/:user_id/FavoriteItems/:id`
  - Marks item as favorite
- `unmark_favorite(item_id)` - DELETE `/Users/:user_id/FavoriteItems/:id`
  - Removes favorite status
- `update_playback_position(item_id, ?PositionTicks)` - POST `/Users/:user_id/PlayingItems/:id/Progress`
  - Updates playback position for resume functionality
  - Position in ticks (10,000 ticks = 1ms)

---

## Data Flow

### Startup Sequence

1. **CLI Parsing** (`bin/main.rs`)
   - Parse `--config` argument (default: `jellofin-server.yaml`)
   - Call `jellofin_rs::run(config_path)`

2. **Configuration Loading** (`lib.rs`)
   - Load YAML config via `Config::from_file()`
   - Validate required fields

3. **Database Initialization**
   - Create `SqliteRepository` with connection pool
   - Run schema migrations
   - Start background flush tasks

4. **Collection Scanning**
   - Create `CollectionRepo` with configured collections
   - Scan all collections: `scan_all()`
   - Build Tantivy search index: `rebuild()`

5. **Image Resizer Setup**
   - Create `ImageResizer` with cache directory
   - Ensure cache directory exists

6. **Server Startup**
   - Build `AppState` with all components
   - Build router with all routes
   - Bind and start HTTP server

### Request Flow

1. **HTTP Request** → axum server
2. **Middleware Chain:**
   - `normalize_path` - Clean URL
   - `log_request` - Log request
   - `CompressionLayer` - Compress response
   - `TraceLayer` - Distributed tracing
3. **Route Matching** → Handler function
4. **Handler Execution:**
   - Extract `State(AppState)`
   - Access database, collections, or image resizer
   - Build response
5. **Response** → Client

### Media Scanning Flow

1. **Trigger:** Startup or manual rescan
2. **For Each Collection:**
   - Determine type (movies/shows)
   - Walk directory tree
   - **If Movies:**
     - One directory = one movie
     - Find video file
     - Find images (poster, fanart, etc.)
     - Parse `movie.nfo` if present
     - Create `Movie` struct
   - **If Shows:**
     - Show directory → Season directories
     - For each season:
       - Find episode video files
       - Parse episode filenames for numbers
       - Find episode images
       - Parse episode `.nfo` files
       - Create `Episode` structs
     - Group episodes into `Season` structs
     - Group seasons into `Show` struct
3. **Store in Memory:**
   - Add to `Collection.movies` or `Collection.shows`
4. **Rebuild Search Index:**
   - Extract searchable fields
   - Create Tantivy documents
   - Write to index

### Image Serving Flow

1. **Request:** `/Images/:item_id/:image_type?width=300&height=200&quality=85`
2. **Lookup Item:**
   - Search all collections for item by ID
   - Get image path from `ImageInfo`
3. **Check Parameters:**
   - If width/height specified → resize
   - Otherwise → serve original
4. **Resize (if needed):**
   - Generate cache key (SHA256)
   - Check cache directory
   - If cached → return cached file
   - If not cached:
     - Load image
     - Resize with Lanczos3
     - Encode with quality settings
     - Save to cache
     - Return resized image
5. **Response:**
   - Set `Content-Type` header
   - Stream file bytes

### Video Streaming Flow

1. **Request:** `/Videos/:id/stream` or `/data/:source/*path`
2. **Lookup Media Source:**
   - Find item by ID
   - Get first media source path
3. **Open File:**
   - Use tokio async file I/O
   - Create `ReaderStream`
4. **Response:**
   - Set `Content-Type: video/x-matroska`
   - Stream file chunks
   - No transcoding (direct stream)

### Search Flow

1. **Request:** `/Search/Hints?SearchTerm=matrix&Limit=20`
2. **Query Tantivy:**
   - Parse search term
   - Query name and overview fields
   - Get top N results
3. **Convert Results:**
   - Map Tantivy documents to `SearchHint` or `BaseItemDto`
4. **Response:**
   - Return `QueryResult<T>` with items and total count

---

## API Routes

### Common Routes

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check endpoint |
| GET | `/System/Ping` | Jellyfin ping endpoint |
| GET | `/robots.txt` | Robots exclusion |
| GET | `/Images/:item_id/:image_type` | Serve item images (with resize) |

### Notflix API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/collections` | List all collections |
| GET | `/api/collection/:id` | Get collection with all items |
| GET | `/api/collection/:id/genres` | Get genre counts for collection |
| GET | `/api/collection/:id/items` | Get collection items (filterable by genre) |
| GET | `/api/collection/:coll_id/item/:item_id` | Get full item details |
| GET | `/data/:source/*path` | Serve media files and images |

**Query Parameters:**
- `/api/collection/:id/items?genre=Action` - Filter by genre
- `/data/:source/*path?width=300&height=200&quality=85` - Image resize

### Jellyfin API

#### Authentication
| Method | Path | Description |
|--------|------|-------------|
| POST | `/Users/AuthenticateByName` | User login (returns token) |

**Request Body:**
```json
{
  "Username": "user",
  "Pw": "password"
}
```

#### System
| Method | Path | Description |
|--------|------|-------------|
| GET | `/System/Info` | Full server information |
| GET | `/System/Info/Public` | Public server information |
| GET | `/Plugins` | Installed plugins (empty) |
| GET | `/DisplayPreferences/usersettings` | Display preferences |

#### Users
| Method | Path | Description |
|--------|------|-------------|
| GET | `/Users` | List all users |
| GET | `/Users/Me` | Get current user |
| GET | `/Users/:user_id/Views` | Get user's library views |
| GET | `/UserViews` | Get library views (alternate) |

#### Library
| Method | Path | Description |
|--------|------|-------------|
| GET | `/Items` | Query items with filters |
| GET | `/Items/:id` | Get item by ID |
| GET | `/Items/Latest` | Get latest items |
| GET | `/Items/Counts` | Get library statistics |

**Query Parameters:**
- `ParentId` - Filter by parent collection
- `Limit` - Maximum results to return

#### Playback
| Method | Path | Description |
|--------|------|-------------|
| POST | `/Items/:id/PlaybackInfo` | Get playback sources |
| GET | `/Videos/:id/stream` | Stream video file (with HTTP Range support) |
| GET | `/Videos/:id/stream.mkv` | Stream video with extension (with HTTP Range support) |
| GET | `/Videos/:id/Subtitles/:index/Stream` | Stream subtitle file by index |
| GET | `/Videos/:id/:index/Subtitles` | Stream subtitle file (alternate route) |

**HTTP Range Support:**
- Video streaming endpoints support `Range` header for seeking
- Returns `206 Partial Content` for range requests
- Returns `200 OK` for full file requests
- Includes `Accept-Ranges: bytes` and proper `Content-Range` headers
- Example: `Range: bytes=0-1023` requests first 1024 bytes

#### Search
| Method | Path | Description |
|--------|------|-------------|
| GET | `/Search/Hints` | Full-text search |
| GET | `/Items/:id/Similar` | Get similar items |

**Query Parameters:**
- `SearchTerm` - Search query
- `Limit` - Maximum results

#### User Data
| Method | Path | Description |
|--------|------|-------------|
| GET | `/Users/:user_id/Items/Resume` | Get items with playback position |
| GET | `/Shows/NextUp` | Get next unwatched episodes |
| POST | `/Users/:user_id/PlayedItems/:id` | Mark item as played |
| DELETE | `/Users/:user_id/PlayedItems/:id` | Mark item as unplayed |
| POST | `/Users/:user_id/FavoriteItems/:id` | Mark item as favorite |
| DELETE | `/Users/:user_id/FavoriteItems/:id` | Remove favorite status |
| POST | `/Users/:user_id/PlayingItems/:id/Progress` | Update playback position |

**Query Parameters:**
- `Limit` - Maximum results (for Resume and NextUp)
- `PositionTicks` - Playback position in ticks (for Progress)

---

## Configuration Example

```yaml
listen:
  address: "0.0.0.0"
  port: 8096
  tlscert: null
  tlskey: null

dbpath: "./jellofin.db"
appdir: "./web"

jellyfin:
  server_name: "My Jellyfin Server"
  server_id: "unique-server-id-12345"
  autoregister: true

collections:
  - id: "movies"
    name: "Movies"
    type: "movies"
    directory: "/media/movies"
    base_url: null
    hls_server: null
  
  - id: "tvshows"
    name: "TV Shows"
    type: "shows"
    directory: "/media/tv"
    base_url: null
    hls_server: null
```

---

## Key Design Decisions

### 1. In-Memory Collections
- All media metadata kept in memory for fast access
- Trade-off: Memory usage vs. query speed
- Acceptable for typical home media libraries (< 100k items)

### 2. Tantivy Search Index
- Full rebuild on startup and after scanning
- Simple implementation, no incremental updates
- Fast enough for typical use cases

### 3. Image Caching
- Disk-based cache with SHA256 keys
- Includes file mtime in key for invalidation
- No cache expiration (manual cleanup via `clear_cache()`)

### 4. No Transcoding
- Direct file streaming only
- Clients must support native formats
- Reduces complexity and CPU usage

### 5. Database Caching
- Access tokens and user data cached in memory
- Periodic flush to SQLite (30s interval)
- Reduces disk I/O for frequently accessed data

### 6. Async Everything
- Tokio runtime for all I/O operations
- Async file reading for streaming
- Async database queries with sqlx

---

## Extension Points

### Adding New Endpoints
1. Add handler function to `notflix/handlers.rs` or `jellyfin/handlers.rs`
2. Add route in `server::build_router()`
3. Add types to `types.rs` if needed

### Adding New Media Types
1. Add item struct to `collection/item.rs`
2. Update scanner in `collection/scanner.rs`
3. Update search index in `collection/search.rs`
4. Add conversion functions in handlers

### Adding New Metadata Sources
1. Add parser to `collection/nfo.rs` or create new module
2. Call parser in `collection/scanner.rs`
3. Map fields to item structs

### Adding Database Tables
1. Add model to `db/model.rs`
2. Add trait to `db/repo.rs`
3. Implement trait in `db/sqlite.rs`
4. Add migration SQL

---

## Performance Considerations

### Bottlenecks
1. **Initial Scan:** O(n) directory walk, can be slow for large libraries
2. **Search Index Rebuild:** O(n) document creation, happens after scan
3. **Image Resizing:** CPU-intensive, but cached after first request
4. **Video Streaming:** I/O bound, limited by disk speed

### Optimizations
1. **Parallel Scanning:** Could parallelize collection scanning
2. **Incremental Search:** Could update index incrementally instead of full rebuild
3. **Image Cache Prewarming:** Could pre-generate common sizes
4. **Database Indexes:** Add indexes for common queries

### Memory Usage
- **Collections:** ~1-2 KB per item (depends on metadata)
- **Search Index:** ~500 bytes per item (Tantivy compressed)
- **Image Cache:** Unbounded (manual cleanup required)
- **Database Cache:** ~100 bytes per token/user data entry

---

## Testing Recommendations

### Integration Tests
1. **Config Loading:** Test YAML parsing with various configurations
2. **Collection Scanning:** Test with sample media directories
3. **API Endpoints:** Test each endpoint with mock data
4. **Image Resizing:** Test various dimensions and formats
5. **Search:** Test query parsing and result ranking

### Client Testing
1. **Jellyfin Web Client:** Browse libraries, play videos
2. **Jellyfin Mobile Apps:** Test authentication and streaming
3. **Infuse (iOS/tvOS):** Test direct play compatibility
4. **Custom Notflix Client:** Test custom API endpoints

### Performance Testing
1. **Load Testing:** Concurrent requests to API endpoints
2. **Large Library:** Test with 10k+ items
3. **Image Cache:** Test cache hit/miss performance
4. **Streaming:** Test multiple concurrent streams

---

## Troubleshooting

### Common Issues

**Server won't start:**
- Check config file path and syntax
- Verify database directory is writable
- Check port availability

**No items in library:**
- Verify collection directory paths
- Check file permissions
- Review scanner logs for errors
- Ensure NFO files are valid XML

**Images not loading:**
- Check image file paths in collection
- Verify cache directory is writable
- Check image file permissions

**Search not working:**
- Verify search index was built (check logs)
- Try rebuilding index (restart server)
- Check Tantivy version compatibility

**Streaming fails:**
- Verify media file paths are correct
- Check file permissions
- Ensure client supports codec/container

---

## Future Enhancements

### Planned Features
1. **Playlist Support:** Create and manage playlists
2. **Metadata Refresh:** Periodic or manual metadata updates
7. **Multi-User Support:** Per-user libraries and permissions
8. **Activity Logging:** Track user viewing history
9. **Remote Images:** Download and cache remote artwork
10. **Live TV:** DVR and live stream support

### Performance Improvements
1. **Incremental Scanning:** Only scan changed files
2. **Parallel Processing:** Multi-threaded scanning and indexing
3. **Database Optimization:** Better indexes and query optimization
4. **Cache Warming:** Pre-generate common image sizes
5. **Streaming Optimization:** Range request support, adaptive bitrate

---

## Dependencies

### Core Dependencies
- `axum` - Web framework
- `tokio` - Async runtime
- `tower` / `tower-http` - Middleware
- `sqlx` - Database access
- `serde` / `serde_json` / `serde_yaml` - Serialization
- `tantivy` - Full-text search
- `image` - Image processing
- `clap` - CLI parsing
- `tracing` / `tracing-subscriber` - Logging
- `chrono` - Date/time handling
- `uuid` - Unique ID generation
- `regex` - Pattern matching
- `sha2` / `hex` - Hashing
- `mime_guess` - MIME type detection
- `tokio-util` - Async utilities
- `thiserror` - Error handling

### Version Constraints
- Tantivy: 0.22 (search index format)
- Image: 0.25 (API compatibility)
- Axum: 0.7 (middleware changes)
- SQLx: 0.8 (async runtime)

---

## Conclusion

This architecture provides a solid foundation for a Jellyfin-compatible media server. The modular design allows for easy extension and modification. The in-memory collection model provides fast access, while the database layer handles persistent state. The dual API support (Jellyfin + Notflix) allows compatibility with existing clients while enabling custom features.

Key strengths:
- **Fast:** In-memory collections, cached images
- **Compatible:** Jellyfin API support
- **Extensible:** Modular design, clear separation of concerns
- **Simple:** No transcoding, direct streaming
- **Rust:** Memory safety, performance, async I/O

The codebase is ready for production use with typical home media libraries and can be extended to support additional features as needed.
