# System Architecture & Codebase Context (`komorebi-server`)

## 1. Overview & Purpose

`komorebi-server` is a high-performance, unified media backend server written in Rust, built on the [Loco](https://loco.rs) 1.1 framework (backed by Axum 0.8 and Sea-ORM 2.0).

The system serves five core functions:

1. **Media Provider Normalization**: Aggregates, synchronizes, and normalizes anime and manga list data across multiple upstream third-party providers (MyAnimeList and AniList) into a single unified REST API.
2. **User & Identity Management**: Manages user profiles with linked provider accounts, optional Argon2 passcode protection, and dual-mode access (token authenticated vs. unauthenticated sandbox mode).
3. **Web Crawler & Torrent Parsing**: Scrapes media/torrent sites (e.g. Nyaa.si) using declarative YAML configs, CSS/JSON selectors, and deep filename parsing via Anitomy.
4. **Multi-Backend Download Manager**: Concurrent acquisition pipeline supporting Direct HTTP/HTTPS downloads with HTTP `Range` resumption, and BitTorrent / Magnet links via `librqbit`.
5. **Vault Storage & Real-time WebSockets**: Manages local media files in UUID-isolated directories, tracks download states in SQLite, and streams live progress, transfer speeds, and ETAs to clients over WebSockets.

---

## 2. Technology Stack

| Layer                 | Technology                      | Description / Usage                                     |
| --------------------- | ------------------------------- | ------------------------------------------------------- |
| **Language**          | Rust (Edition 2024)             | Modern Rust with strict compiler checks                 |
| **Web Framework**     | [Loco](https://loco.rs) 1.1     | Rails-inspired batteries-included Rust framework        |
| **Web & WS**          | Axum 0.8                        | Async HTTP routing and WebSocket connection upgrades    |
| **Async Runtime**     | Tokio 1.53                      | Multi-threaded runtime, tasks, broadcast channels       |
| **Database**          | SQLite via Sea-ORM 2.0          | Async connection pooling, schema migrations, and ORM    |
| **HTTP Client**       | reqwest 0.13                    | Connection pooled HTTP client with query/JSON/form      |
| **BitTorrent Engine** | librqbit 9.0                    | Embedded BitTorrent engine with DHT session persistence |
| **Identifiers**       | UUID v7 (`uuid` crate)          | Time-sortable primary keys for all database entities    |
| **Timestamps**        | chrono 0.4                      | Millisecond-precision timestamps (fixed offset / UTC)   |
| **HTML Parsing**      | scraper 0.27                    | CSS selector extraction for web scraping                |
| **Title Parsing**     | anitomy-rs                      | Torrent filename tokenizer and anime metadata extractor |
| **Serialization**     | serde / serde_json / yaml_serde | Strong typing across JSON APIs and YAML configurations  |
| **Enum Tools**        | strum / strum_macros 0.28       | String serialization and case-insensitive parsing       |
| **TS Bindings**       | ts-rs 12                        | Emits TypeScript interface definitions to frontend      |

---

## 3. Codebase Directory Layout

```
komorebi-server/
├── assets/                          # SQLite databases, crawler configs, DHT session cache
│   ├── crawler_configs.yaml         # Active scraper configurations (Nyaa, etc.)
│   ├── dht.json                     # librqbit persistent DHT node cache
│   └── main.sqlite                  # Default SQLite database
├── config/                          # Loco per-environment configuration YAMLs
│   ├── development.yaml             # Development port (5150), log levels, db paths
│   ├── production.yaml              # Production environment overrides
│   └── test.yaml                    # Isolated in-memory test database setup
├── docs/
│   ├── CONTEXT.md                   # This file — master architectural guide
│   ├── openapi.yaml                 # OpenAPI 3.1 REST API specification
│   ├── schema.sql                   # Reference SQLite schema DDL
│   └── *.schema.json                # Upstream MAL & AniList schema references
├── migration/                       # Sea-ORM database migrations
│   ├── src/lib.rs                   # Migrator registration
│   ├── src/m20220101_000001_users.rs  # Users table & unique composite index
│   └── src/m20260825_072131_vaults.rs # Vault table, foreign keys & indexes
├── src/
│   ├── lib.rs                       # Crate root exporting all submodules
│   ├── app.rs                       # Loco Hooks — route registration, shared state, hooks
│   ├── bin/
│   │   ├── main.rs                  # CLI entrypoint (`komorebi_server-cli`)
│   │   └── tool.rs                  # Secondary tool binary
│   ├── adapters/                    # Upstream provider client abstractions
│   │   ├── mod.rs                   # MediaClient trait & MediaProvider dispatch
│   │   ├── mal_client.rs            # MyAnimeList REST API client
│   │   ├── mal_models.rs            # MAL DTO response models & normalization
│   │   ├── anilist_client.rs        # AniList GraphQL API client
│   │   └── anilist_models.rs        # AniList GraphQL DTO models & normalization
│   ├── controllers/                 # Axum / Loco HTTP controllers & routes
│   │   ├── mod.rs                   # Standard success() and fail() envelope helpers
│   │   ├── user_controller.rs       # /api/v1/user/* endpoints
│   │   ├── media_controller.rs      # /api/v1/media/* endpoints
│   │   ├── crawler_controller.rs    # /api/v1/crawler/* endpoints
│   │   └── vault_controller.rs      # /api/v1/vault/* endpoints & /vault/ws upgrade
│   ├── crawlers/                    # Web scraping & title parsing subsystem
│   │   ├── mod.rs                   # Crawler & TitleParser traits
│   │   ├── crawler_engine.rs        # Concurrent multi-source crawl orchestrator
│   │   ├── html_crawler.rs          # Scraper-based HTML extraction
│   │   ├── json_crawler.rs          # JSON API extraction
│   │   ├── config_parser.rs         # YAML crawler config loader & lazy static
│   │   └── anitomy_title_parser.rs  # Anitomy-rs filename parsing implementation
│   ├── downloaders/                 # Multi-backend download management
│   │   ├── mod.rs                   # DownloadEngine trait definition
│   │   ├── manager.rs               # DownloadManager orchestrator singleton
│   │   ├── direct.rs                # DirectDownloader (HTTP Range resume & streaming)
│   │   ├── torrent.rs               # TorrentDownloader (librqbit session management)
│   │   └── daemon.rs                # Background polling daemon & WebSocket pump
│   ├── models/                      # Sea-ORM active models & domain types
│   │   ├── _entities/               # GENERATED Sea-ORM entities (do not edit)
│   │   │   ├── users.rs             # Generated users schema entity
│   │   │   └── vault.rs             # Generated vault schema entity
│   │   ├── mod.rs                   # Model re-exports
│   │   ├── users.rs                 # User entity logic, ActiveModelBehavior, Argon2 auth
│   │   ├── media.rs                 # Domain entities: Media, ListEntry, MediaEntry, enums
│   │   ├── crawler.rs               # CrawlerConfig, CrawlerResult, ParsedTitle types
│   │   └── vault.rs                 # VaultItem logic, VaultDownloadType, VaultItemStatus
│   ├── core/                        # Shared utilities, constants, path resolvers
│   │   ├── mod.rs                   # ResultExt / ResultStringExt error conversion traits
│   │   ├── client.rs                # Shared reqwest::Client & public tracker fetcher
│   │   ├── constants.rs             # VAULT_LOC, auth URLs, static configs
│   │   └── vault_path_resolver.rs   # Per-item destination path generator
│   ├── dtos/                        # TypeScript-exported DTO types (ts-rs)
│   │   └── mod.rs                   # DTO exports
│   └── workers/                     # Loco background job workers
│       ├── mod.rs                   # Worker module exports
│       └── downloader.rs            # Loco queue DownloadWorker
└── tests/                           # Unit & request integration test suites
    ├── adapters/                    # MAL & AniList model mapping tests
    ├── crawlers/                    # HTML/JSON scraper & parser tests
    ├── models/                      # User & Vault model behavior tests
    └── requests/                    # Controller HTTP endpoint tests
```

---

## 4. Domain Data Models

### 4.1 Media Domain Types (`src/models/media.rs`)

- **`MediaProvider`**: Supported third-party media sources (`MAL`, `ANILIST`). Stored as uppercase string in database.
- **`MediaType`**: Type of media item (`Anime`, `Manga`, `Novel`). Supports case-insensitive deserialization.
- **`MediaFormat`**: Unified release medium across MAL and AniList (`Unknown`, `Tv`, `TvShort`, `Movie`, `Special`, `Ova`, `Ona`, `Music`, `Manga`, `Novel`, `OneShot`, `Doujinshi`, `Manhwa`, `Manhua`, `Oel`).
- **`ReleaseStatus`**: Airing/publishing state (`Unknown`, `Releasing`, `Finished`, `NotYetReleased`, `Cancelled`, `Hiatus`).
- **`ListStatus`**: User list tracking status (`Current`, `Planning`, `Completed`, `Dropped`, `Paused`, `Repeating`).
- **`NsfwLevel`**: Maturity rating (`Safe`, `Gray`, `Nsfw`).
- **`Media`**: Provider-agnostic metadata representing an anime or manga title (`id`, `provider_id`, `provider`, `media_type`, `format`, `release_status`, `title: MediaTitle`, `cover: CoverImage`, `synopsis`, `mean_score: Option<f32>`, `popularity`, `episodes`, `duration`, `chapters`, `volumes`, `genres`, `nsfw`).
- **`ListEntry`**: User-specific tracking metrics (`status`, `score` normalized to 0.0–10.0 scale, `progress`, `progress_volumes`, `is_repeating`, `repeat_count`, `tags`, `notes`, `updated_at`).
- **`MediaEntry`**: Pair of `(Media, ListEntry)` returned as list items.
- **`PaginatedResponse`**: Response payload containing `Vec<MediaEntry>` and `Paging` (`next_cursor`, `prev_cursor`, `has_next`).

### 4.2 User Model (`src/models/users.rs`)

`User` is a type alias for `models::_entities::users::Model`.

- **Fields**: `id` (UUID v7), `username`, `provider_id`, `avatar_url`, `provider` (`MediaProvider`), `is_sandbox` (`bool`), `access_token` (`Option<String>`), `passcode` (`Option<String>`), `created_at`, `updated_at`.
- **`ActiveModelBehavior::before_save`**:
  - Automatically assigns a new `Uuid::now_v7()` if the ID is nil.
  - Automatically calculates `is_sandbox = access_token.is_none() || access_token.is_empty()`.
  - Sets `created_at` on insert and updates `updated_at` on every save.
- **`save_user`**: UPSERT on composite key conflict `(username, provider, is_sandbox)`. Updates `access_token`, `is_sandbox`, `avatar_url`, and `updated_at`.
- **`verify_passcode`**: Verifies password against stored passcode. Uses Argon2 hash verification (`hash::verify_password`) when the string begins with `$argon2`, falling back to plaintext comparison for non-hashed legacy passcodes.

### 4.3 Crawler Domain Types (`src/models/crawler.rs`)

- **`CrawlerConfig`**: Scraper definition loaded from YAML (`id`, `name`, `base_url`, `item_selector`, `title_selector`, `link_selector`, `popularity_selector`, `size_selector`, `is_active`, `category`). Provides `CrawlerConfig::fallback()` for Nyaa.si.
- **`CrawlerResult`**: Single scraped item (`title`, `link`, `source`, `popularity`, `size`, `parsed_title: ParsedTitle`, `category: MediaType`).
- **`ParsedTitle`**: Structured fields extracted by Anitomy (`title`, `season`, `episode`, `video_resolution`, `release_group`, `subtitles`, `audio_term`, `file_extension`, `kind`, etc.). Uses `IndexSet<String>` for deterministic order and uniqueness.

### 4.4 Vault Domain Types (`src/models/vault.rs`)

`VaultItem` is a type alias for `models::_entities::vault::Model`.

- **Fields**: `id` (UUID v7), `user_id` (UUID v7), `destination_path` (`String`), `media_type` (`Option<MediaType>`), `media_id` (`String`), `title` (`String`), `raw_title` (`String`), `season` (`Option<String>`), `episode` (`Option<String>`), `source_url` (`String`), `download_type` (`VaultDownloadType`), `status` (`VaultItemStatus`), `total_bytes` (`i64`), `downloaded_bytes` (`i64`), `progress` (`f64`), `speed_bps` (`i64`), `eta_seconds` (`Option<i64>`), `temp_path` (`String`), `error_msg` (`Option<String>`), `created_at`, `updated_at`.
- **`VaultDownloadType`**: `DIRECT` (HTTP/HTTPS URL), `MAGNET` (BitTorrent magnet URI), `TFILE` (Local or remote `.torrent` file).
- **`VaultItemStatus`**: `PENDING`, `DOWNLOADING`, `PAUSED`, `COMPLETED`, `FAILED`, `CANCELLED`.
- **`ActiveModelBehavior::before_save`**: Auto-assigns `Uuid::now_v7()` if nil and updates `updated_at`.

---

## 5. Persistence Layer & Database Schema

The database is SQLite managed asynchronously through Sea-ORM.

### 5.1 Tables & Constraints

#### `users` Table

```sql
CREATE TABLE IF NOT EXISTS users (
    id           BLOB    PRIMARY KEY NOT NULL, -- UUID v7
    username     TEXT    NOT NULL,
    provider_id  TEXT,
    avatar_url   TEXT,
    provider     TEXT    NOT NULL,
    is_sandbox   BOOLEAN NOT NULL DEFAULT 1,
    access_token TEXT,
    passcode     TEXT,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    updated_at   INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000)
);

CREATE UNIQUE INDEX `idx-uniq-users-username-provider-is_sandbox`
ON users (username, provider, is_sandbox);
```

#### `vault` Table

```sql
CREATE TABLE IF NOT EXISTS vault (
    id                BLOB    PRIMARY KEY NOT NULL, -- UUID v7
    user_id           BLOB    NOT NULL,             -- FK to users(id) ON DELETE CASCADE
    destination_path  TEXT    NOT NULL UNIQUE,      -- {VAULT_LOC}/{vault_id}/
    media_type        TEXT    NOT NULL DEFAULT 'ANIME',
    media_id          TEXT,
    title             TEXT    NOT NULL,
    raw_title         TEXT    NOT NULL,
    season            TEXT,
    episode           TEXT,
    source_url        TEXT    NOT NULL,
    download_type     TEXT    NOT NULL DEFAULT 'MAGNET',
    status            TEXT    NOT NULL DEFAULT 'PENDING',
    total_bytes       INTEGER NOT NULL DEFAULT 0,
    downloaded_bytes  INTEGER NOT NULL DEFAULT 0,
    progress          REAL    NOT NULL DEFAULT 0.0,
    speed_bps         INTEGER NOT NULL DEFAULT 0,
    eta_seconds       INTEGER,
    temp_path         TEXT    NOT NULL,
    error_msg         TEXT,
    created_at        INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    updated_at        INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_vault_user_id ON vault(user_id);
CREATE INDEX idx_vault_status  ON vault(status);
```

---

## 6. HTTP API & Controller Architecture

All application endpoints are prefixed with `/api/v1`.

### 6.1 Standard Response Envelopes (`src/controllers/mod.rs`)

All JSON responses strictly follow standardized envelope structures:

- **Success (`200 OK`)**:
  ```json
  {
    "success": true,
    "data": { ... }
  }
  ```
- **Failure (`4xx / 5xx`)**:
  ```json
  {
    "success": false,
    "error": "ERROR_CODE",
    "description": "Optional human-readable error description"
  }
  ```

### 6.2 Endpoints Summary

| Controller  | Method | Path                          | Description                                                        |
| :---------- | :----- | :---------------------------- | :----------------------------------------------------------------- |
| **User**    | `POST` | `/api/v1/user/login`          | Authenticate by username, provider, sandbox flag, and passcode     |
| **User**    | `POST` | `/api/v1/user/add`            | Validate user against upstream provider and upsert record          |
| **User**    | `POST` | `/api/v1/user/all`            | Return all registered users in database                            |
| **User**    | `POST` | `/api/v1/user/one`            | Fetch single user by UUID                                          |
| **User**    | `POST` | `/api/v1/user/delete`         | Delete user by UUID (cascades to user's vault items)               |
| **User**    | `POST` | `/api/v1/user/oauth/exchange` | Exchange OAuth authorization code + PKCE verifier for access token |
| **Media**   | `POST` | `/api/v1/media/anime`         | Fetch user's anime list with pagination and filtering              |
| **Media**   | `POST` | `/api/v1/media/manga`         | Fetch user's manga list with pagination and filtering              |
| **Crawler** | `POST` | `/api/v1/crawler/search`      | Search external torrent/media crawlers and parse titles            |
| **Vault**   | `POST` | `/api/v1/vault/add`           | Add download item to vault and dispatch to download engine         |
| **Vault**   | `POST` | `/api/v1/vault/one`           | Fetch vault item by UUID                                           |
| **Vault**   | `POST` | `/api/v1/vault/all`           | List all vault items in database                                   |
| **Vault**   | `POST` | `/api/v1/vault/pause`         | Pause active direct or torrent download                            |
| **Vault**   | `POST` | `/api/v1/vault/resume`        | Resume paused download                                             |
| **Vault**   | `POST` | `/api/v1/vault/delete`        | Cancel/delete download task and delete files from disk             |
| **Vault**   | `GET`  | `/api/v1/vault/ws`            | WebSocket upgrade endpoint streaming real-time download progress   |

---

## 7. Upstream Provider Adapter Layer (`src/adapters/`)

### 7.1 `MediaClient` Trait

```rust
#[async_trait]
pub trait MediaClient: Send + Sync {
    fn new(client: &reqwest::Client, user: &User) -> Self where Self: Sized;
    async fn get_anime_list(&self, params: &MediaClientParams) -> Result<PaginatedResponse>;
    async fn get_manga_list(&self, params: &MediaClientParams) -> Result<PaginatedResponse>;
    async fn validate_new_user(&self, access_token: &str) -> Result<User>;
    async fn exchange_oauth_token(&self, code: &str, code_verifier: &str) -> Result<String>;
}
```

Use `user.provider.new_client(client, user)` to dynamically instantiate `Box<dyn MediaClient>`.

### 7.2 Implementations

- **`MalClient`**: Queries MyAnimeList REST API (`/v2/users/{username}/animelist`, `/v2/users/{username}/mangalist`). Maps scores directly (0.0–10.0) and uses cursor-based pagination.
- **`AniListClient`**: Queries AniList GraphQL API (`https://graphql.anilist.co`). Normalizes 0–100 scores to 0.0–10.0 and converts page-based pagination.

---

## 8. Web Crawler Subsystem (`src/crawlers/`)

The crawler subsystem discovers media downloads from external indexers without hardcoding site logic into controllers:

1. **Config Loader (`config_parser.rs`)**: Reads `assets/crawler_configs.yaml` at startup into a `LazyLock<Vec<Arc<CrawlerConfig>>>`.
2. **Crawler Engine (`crawler_engine.rs`)**: Dispatches concurrent HTTP scrape tasks across all active configurations matching the requested `MediaType`.
3. **Scrapers**:
   - `HtmlCrawler`: Uses `scraper` with CSS selectors (`item_selector`, `title_selector`, `link_selector`, `popularity_selector`, `size_selector`).
   - `JsonCrawler`: Maps JSON keys directly into results.
4. **Title Parser (`anitomy_title_parser.rs`)**: Runs raw release titles through `anitomy-rs` to reliably extract anime title, season, episode, video resolution, release group, audio terms, and subtitles.

---

## 9. Vault & Download Engine Subsystem (`src/downloaders/`)

The download management subsystem provides robust, multi-backend file acquisition with real-time feedback:

```
                  ┌──────────────────────┐
                  │   POST /vault/add    │
                  └──────────┬───────────┘
                             │
                             ▼
                  ┌──────────────────────┐
                  │   DownloadManager    │
                  └──────────┬───────────┘
            ┌────────────────┴────────────────┐
            ▼                                 ▼
┌──────────────────────┐          ┌──────────────────────┐
│   DirectDownloader   │          │  TorrentDownloader   │
│ (HTTP Range Resume)  │          │   (librqbit engine)  │
└──────────┬───────────┘          └──────────┬───────────┘
           │                                 │
           └────────────────┬────────────────┘
                            │ (Live Stats)
                            ▼
               ┌────────────────────────┐
               │ Background Daemon Loop │ ◄── Woken on demand via Notify
               └────────────┬───────────┘
                     ┌──────┴──────┐
                     ▼             ▼
              ┌────────────┐ ┌────────────┐
              │ SQLite DB  │ │ WebSocket  │ (/vault/ws)
              │ (Persist)  │ │ (Broadcast)│
              └────────────┘ └────────────┘
```

### 9.1 Core Components

- **`DownloadEngine` Trait (`downloaders/mod.rs`)**:
  Defines `add()`, `pause()`, `resume()`, `delete()`, `get_stats()`, and `stop()` for download engines.
- **`DownloadManager` (`downloaders/manager.rs`)**:
  - Singleton stored in `AppContext.shared_store`.
  - Owns an `active_items: Arc<DashMap<Uuid, VaultItem>>` in-memory map.
  - Initializes persistent `librqbit::Session` (using `assets/dht.json`).
  - Fetches public BitTorrent trackers from GitHub tracker lists via `get_common_trackers()`.
  - On startup, queries SQLite for any incomplete (`PENDING` or `DOWNLOADING`) items and automatically re-queues them across download engines.
- **`TorrentDownloader` (`downloaders/torrent.rs`)**:
  - Wraps `librqbit::Session` and manages `ManagedTorrent` handles.
  - Handles `MAGNET` links and `TFILE` torrent files.
  - Updates progress bytes, speeds, and completion states.
- **`DirectDownloader` (`downloaders/direct.rs`)**:
  - Streams HTTP/HTTPS files directly into `{destination_path}/S{season} EP{episode} {title}`.
  - Automatically inspects existing file size on disk and sends `Range: bytes={downloaded_bytes}-` for resumable downloads.
  - Computes rolling transfer speeds (`speed_bps`) and ETAs (`eta_seconds`).
  - Uses `tokio_util::sync::CancellationToken` for non-blocking pause/cancel operations.
- **Background Daemon (`downloaders/daemon.rs`)**:
  - Runs in a background Tokio task.
  - Polls engine stats every 2 seconds when downloads are active.
  - Flushes progress to SQLite via `vault::ActiveModel::update_progress_mut()`.
  - Emits stats to the Tokio broadcast channel (`Sender<Vec<VaultItem>>`).
  - Enters sleep when no downloads are active and wakes instantly when `DownloadManager::wake_daemon()` is triggered.
- **Real-Time WebSockets (`controllers/vault_controller.rs`)**:
  - `GET /api/v1/vault/ws` upgrades incoming connections to WebSockets.
  - Immediately transmits the current list of active downloads on connect.
  - Subscribes to the broadcast channel and streams real-time updates as JSON frames.

---

## 10. Shared State & Application Lifecycle (`src/app.rs`)

### 10.1 `AppContext.shared_store`

The following shared singletons are registered in `App::after_context()`:

1. `reqwest::Client`: Unified HTTP client configured with pooled connections and custom headers.
2. `Arc<DownloadManager>`: Global download orchestrator and engine dispatcher.
3. `tokio::sync::broadcast::Sender<Vec<VaultItem>>`: Broadcast channel for WebSocket progress events.

### 10.2 Graceful Shutdown (`on_shutdown`)

When the application receives a termination signal (`SIGINT` / `SIGTERM`), `App::on_shutdown()` is invoked:

- Iterates over all active download engines from `DownloadManager::get_all_engines()`.
- Calls `engine.stop()` wrapped in a 5-second timeout to flush torrent state and DHT nodes cleanly.

---

## 11. Development & Testing Workflow

### 11.1 Essential Commands

```sh
# Apply database migrations
cargo loco db migrate

# Start the development server (http://localhost:5150)
cargo loco start

# List all registered routes
cargo loco routes

# Verify environment configuration and doctor check
cargo loco doctor

# Run the complete test suite
cargo test
```

### 11.2 Environment Variables

| Variable                | Description                               | Default                                 |
| :---------------------- | :---------------------------------------- | :-------------------------------------- |
| `PORT`                  | Server listening port                     | `5150`                                  |
| `BINDING`               | Server bind host interface                | `localhost`                             |
| `DATABASE_URL`          | SQLite database URI                       | `sqlite://assets/main.sqlite?mode=rwc`  |
| `QUEUE_URL`             | SQLite queue URI                          | `sqlite://assets/queue.sqlite?mode=rwc` |
| `VAULT_LOC`             | Root directory for downloaded vault files | `vault`                                 |
| `MAL_CLIENT_ID`         | MyAnimeList API Client ID                 | (Optional for OAuth)                    |
| `MAL_CLIENT_SECRET`     | MyAnimeList API Client Secret             | (Optional for OAuth)                    |
| `ANILIST_CLIENT_ID`     | AniList API Client ID                     | (Optional for OAuth)                    |
| `ANILIST_CLIENT_SECRET` | AniList API Client Secret                 | (Optional for OAuth)                    |
