# System Architecture & Codebase Context (`komorebi-server`)

## 1. Overview & Purpose

`komorebi-server` is a high-performance, unified media backend server written in Rust, built on the [Loco](https://loco.rs) 1.1 framework (backed by Axum 0.8 and Sea-ORM 2.0).

The system serves six core functions:

1. **Media Provider Normalization**: Aggregates, synchronizes, and normalizes anime and manga list data across multiple upstream third-party providers (MyAnimeList and AniList) into a single unified REST API.
2. **User & Identity Management**: Manages user profiles with linked provider accounts, optional Argon2 passcode protection, and dual-mode access (token authenticated vs. unauthenticated sandbox mode).
3. **Web Crawler & Torrent Parsing**: Scrapes media/torrent sites (e.g. Nyaa.si) using declarative YAML configs, CSS/JSON selectors, and deep filename parsing via Anitomy.
4. **Multi-Backend Download Manager**: Concurrent acquisition pipeline supporting Direct HTTP/HTTPS downloads with HTTP `Range` resumption, and BitTorrent / Magnet links via `librqbit`.
5. **Media Post-Processing & Transcoding**: Automated post-processing pipeline utilizing FFmpeg and ffprobe to inspect media, generate video streams/manifests, extract audio tracks, subtitles (ASS, SRT, WebVTT), embedded font attachments, chapter markers, and video thumbnails.
6. **Vault Storage, Video Streaming & Real-Time SSE**: Manages local media files in UUID-isolated directories, serves range-compatible video streaming (`/api/v1/vault/stream/{*path}`), metadata querying (`/api/v1/vault/metadata`), and streams live progress, transfer speeds, and ETAs to clients via Server-Sent Events (SSE).

---

## 2. Technology Stack

| Layer                  | Technology                                      | Description / Usage                                         |
| ---------------------- | ----------------------------------------------- | ----------------------------------------------------------- |
| **Language**           | Rust (Edition 2024)                             | Modern Rust with strict compiler checks                     |
| **Web Framework**      | [Loco](https://loco.rs) 1.1                     | Rails-inspired batteries-included Rust framework            |
| **Web & Streaming**    | Axum 0.8                                        | Async HTTP routing, Server-Sent Events (SSE), `ServeFile`   |
| **Async Runtime**      | Tokio 1.53                                      | Multi-threaded runtime, tasks, broadcast channels           |
| **Database**           | SQLite via Sea-ORM 2.0                          | Async connection pooling, schema migrations, and ORM        |
| **HTTP Client**        | reqwest 0.13                                    | Connection pooled HTTP client with query/JSON/form          |
| **BitTorrent Engine**  | librqbit 9.0                                    | Embedded BitTorrent engine with DHT session persistence     |
| **Media Transcoding**  | FFmpeg, FFprobe (`rust_ffmpeg`, `rust_ffprobe`) | Video probing, stream extraction, transcoding, thumbnailing |
| **Identifiers**        | UUID v7 (`uuid` crate)                          | Time-sortable primary keys for all database entities        |
| **Timestamps**         | chrono 0.4                                      | Millisecond-precision timestamps (fixed offset / UTC)       |
| **HTML Parsing**       | scraper 0.27                                    | CSS selector extraction for web scraping                    |
| **Title Parsing**      | anitomy-rs                                      | Torrent filename tokenizer and anime metadata extractor     |
| **Serialization**      | serde / serde_json / yaml_serde                 | Strong typing across JSON APIs and YAML configurations      |
| **Caching**            | cached 3.1                                      | In-memory function memoization and path checks              |
| **Enum Tools**         | strum / strum_macros 0.28                       | String serialization and case-insensitive parsing           |
| **TS Bindings**        | ts-rs 12                                        | Emits TypeScript interface definitions to frontend          |

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
│   ├── schema.sql                   # Reference SQLite schema DDL (8 tables)
│   └── *.schema.json                # Upstream MAL & AniList schema references
├── migration/                       # Sea-ORM database migrations
│   ├── src/lib.rs                   # Migrator registration
│   ├── src/m20220101_000001_initial_schema.rs # Initial migration script
│   └── schema.sql                   # Executed migration SQL script
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
│   │   ├── vault_controller.rs      # /api/v1/vault/* lifecycle & SSE endpoints
│   │   └── vault_stream.rs          # /api/v1/vault/stream & /api/v1/vault/metadata endpoints
│   ├── streaming/                   # Video transcoding, extraction, and stream serving
│   │   ├── mod.rs                   # PostProcessor trait & StreamingEvent enum
│   │   ├── processor.rs             # MediaProcessor orchestrator & path resolvers
│   │   ├── video.rs                 # VideoProcessor (FFmpeg/ffprobe pipeline)
│   │   └── daemon.rs                # Background monitoring daemon for transcoding progress
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
│   │   └── daemon.rs                # Background polling daemon & stats flusher
│   ├── models/                      # Sea-ORM active models & entities
│   │   ├── _entities/               # GENERATED Sea-ORM entities (do not edit)
│   │   ├── mod.rs                   # Model re-exports
│   │   ├── users.rs                 # User entity logic, ActiveModelBehavior, Argon2 auth
│   │   ├── vault.rs                 # VaultItem logic, VaultDownloadType, VaultStatus
│   │   ├── vault_sub_item.rs        # VaultSubItem model (individual media files)
│   │   ├── vault_metadata.rs        # VaultMetadata model (probed media info & thumbnails)
│   │   ├── audio_tracks.rs          # AudioTrack entity (extracted audio streams)
│   │   ├── video_subtitles.rs       # VideoSubtitle entity (extracted subtitles)
│   │   ├── video_chapters.rs        # VideoChapter entity (extracted chapter markers)
│   │   └── subtitle_fonts.rs        # SubtitleFont entity (extracted font attachments)
│   ├── dtos/                        # Domain types & TypeScript-exported DTOs (ts-rs)
│   │   ├── mod.rs                   # DTO re-exports
│   │   ├── crawler.rs               # CrawlerConfig, CrawlerResult, ParsedTitle types
│   │   ├── enums.rs                 # VaultStatus, MediaType, MediaFormat, ListStatus, etc.
│   │   ├── events.rs                # AppEvent & SSE serialization
│   │   ├── media.rs                 # Media, ListEntry, MediaEntry, PaginatedResponse
│   │   └── vault.rs                 # VaultSubItemDto, VaultMetadataDto
│   ├── core/                        # Shared utilities, constants, path resolvers
│   │   ├── mod.rs                   # ResultExt / ResultStringExt error conversion traits
│   │   ├── client.rs                # Shared reqwest::Client & public tracker fetcher
│   │   ├── constants.rs             # VAULT_LOC, ENCODED_LOC, MANIFEST_LOC, FONTS_LOC, SUBS_LOC
│   │   └── vault_path_resolver.rs   # Per-item destination path generator
│   └── workers/                     # Loco background job workers
│       ├── mod.rs                   # Worker module exports
│       └── downloader.rs            # Loco queue DownloadWorker
└── tests/                           # Unit & request integration test suites
    ├── adapters/                    # MAL & AniList model mapping tests
    ├── crawlers/                    # HTML/JSON scraper & parser tests
    ├── models/                      # User & Vault model behavior tests
    ├── requests/                    # Controller HTTP endpoint tests
    └── streaming/                   # Media extraction and video processing tests
```

---

## 4. Domain Data Models & DTOs

### 4.1 Media Domain Types (`src/dtos/media.rs`)

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

### 4.3 Crawler Domain Types (`src/dtos/crawler.rs`)

- **`CrawlerConfig`**: Scraper definition loaded from YAML (`id`, `name`, `base_url`, `item_selector`, `title_selector`, `link_selector`, `popularity_selector`, `size_selector`, `is_active`, `category`). Provides `CrawlerConfig::fallback()` for Nyaa.si.
- **`CrawlerResult`**: Single scraped item (`title`, `link`, `source`, `popularity`, `size`, `parsed_title: ParsedTitle`, `category: MediaType`).
- **`ParsedTitle`**: Structured fields extracted by Anitomy (`title`, `season`, `episode`, `video_resolution`, `release_group`, `subtitles`, `audio_term`, `file_extension`, `kind`, etc.). Uses `IndexSet<String>` for deterministic order and uniqueness.

### 4.4 Vault Item Model (`src/models/vault.rs`)

`VaultItem` is a type alias for `models::_entities::vault::Model`.

- **Fields**: `id` (UUID v7), `user_id` (UUID v7), `dest_path` (`String`), `media_type` (`MediaType`), `media_id` (`Option<String>`), `title` (`String`), `raw_title` (`String`), `source_url` (`String`), `download_type` (`VaultDownloadType`), `status` (`VaultStatus`), `total_bytes` (`i64`), `downloaded_bytes` (`i64`), `progress` (`f64`), `speed_bps` (`i64`), `eta_seconds` (`Option<i64>`), `error_msg` (`Option<String>`), `created_at`, `updated_at`.
- **`VaultDownloadType`**: `DIRECT` (HTTP/HTTPS URL), `MAGNET` (BitTorrent magnet URI), `TFILE` (Local or remote `.torrent` file).
- **`VaultStatus`**: `PENDING`, `DOWNLOADING`, `PROCESSING`, `READY`, `PAUSED`, `COMPLETED`, `FAILED`, `CANCELLED`.
- **`ActiveModelBehavior::before_save`**: Auto-assigns `Uuid::now_v7()` if nil and updates `updated_at`.

### 4.5 Vault Sub-Item Model (`src/models/vault_sub_item.rs`)

`VaultSubItem` represents individual downloadable files discovered within a vault item's destination folder (e.g. multi-episode torrents or archives).

- **Fields**: `id` (UUID v7), `vault_id` (UUID v7, FK to `vault.id`), `source_path` (`String`), `dest_path` (`Option<String>`), `media_type` (`MediaType`), `media_id` (`Option<String>`), `title` (`String`), `raw_title` (`String`), `season` (`Option<String>`), `episode` (`Option<String>`), `status` (`VaultStatus`), `total_bytes` (`i64`), `progress` (`f64`), `speed_bps` (`i64`), `eta_seconds` (`Option<i64>`), `error_msg` (`Option<String>`), `created_at`, `updated_at`.

### 4.6 Relational Video Metadata Models

- **`VaultMetadata` (`src/models/vault_metadata.rs`)**: Probe summary linked 1:1 to `VaultSubItem` (`id`, `sub_item_id`, `file_name`, `file_path`, `thumbnail_path`, timestamps).
- **`AudioTrack` (`src/models/audio_tracks.rs`)**: Audio stream extracted from video (`id`, `metadata_id`, `language`, `title`, `channels`, `is_default`, timestamps).
- **`VideoSubtitle` (`src/models/video_subtitles.rs`)**: Subtitle track extracted from media (`id`, `metadata_id`, `track`, `language`, `title`, `format`, `file_path`, `is_forced`, timestamps).
- **`VideoChapter` (`src/models/video_chapters.rs`)**: Chapter marker (`id`, `metadata_id`, `chapter_id`, `title`, `start_time`, `end_time`, timestamps).
- **`SubtitleFont` (`src/models/subtitle_fonts.rs`)**: Embedded font extracted from MKV attachments (`id`, `metadata_id`, `font_name`, `file_path`, timestamps).

### 4.7 DTOs & Events (`src/dtos/`)

- **`VaultSubItemDto` (`src/dtos/vault.rs`)**: Wraps `VaultSubItem` and optional `VaultMetadataDto`.
- **`VaultMetadataDto` (`src/dtos/vault.rs`)**: Contains `VaultMetadata` along with vectors of `AudioTrack`, `VideoSubtitle`, `VideoChapter`, and `SubtitleFont`.
- **`AppEvent` (`src/dtos/events.rs`)**: Unified event envelope serialized to Server-Sent Events (SSE). Variants: `Unknown`, `VaultItems(Vec<VaultItem>)`, `VaultSubItems(Vec<VaultSubItem>)`, `StreamingEvents(StreamingEvent)`, `Error(String)`.
- **`StreamingEvent` (`src/streaming/mod.rs`)**: Real-time post-processing progress updates:
  - `Init { vault_id, total }`
  - `Progress { sub_item_id, total_size, speed, progress, eta_secs }`
  - `Complete { sub_item_id, vault_id, is_last }`

---

## 5. Persistence Layer & Database Schema

The database is SQLite managed asynchronously through Sea-ORM. The schema consists of 8 tables with foreign keys and automatic `updated_at` triggers:

```sql
PRAGMA foreign_keys = ON;

-- 1. users
CREATE TABLE IF NOT EXISTS users (
    id           BLOB PRIMARY KEY NOT NULL,
    username     TEXT NOT NULL,
    provider_id  TEXT,
    avatar_url   TEXT,
    provider     TEXT NOT NULL,
    is_sandbox   INTEGER NOT NULL DEFAULT 1,
    access_token TEXT,
    passcode     TEXT,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(username, provider, is_sandbox)
);

-- 2. vault
CREATE TABLE IF NOT EXISTS vault (
    id               BLOB PRIMARY KEY NOT NULL,
    user_id          BLOB NOT NULL,
    dest_path        TEXT NOT NULL,
    media_type       TEXT NOT NULL DEFAULT 'ANIME',
    media_id         TEXT,
    title            TEXT NOT NULL,
    raw_title        TEXT NOT NULL,
    source_url       TEXT NOT NULL,
    download_type    TEXT NOT NULL DEFAULT 'MAGNET',
    status           TEXT NOT NULL DEFAULT 'PENDING',
    total_bytes      INTEGER NOT NULL DEFAULT 0,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    progress         REAL NOT NULL DEFAULT 0.0,
    speed_bps        INTEGER NOT NULL DEFAULT 0,
    eta_seconds      INTEGER,
    error_msg        TEXT,
    created_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 3. vault_sub_item
CREATE TABLE IF NOT EXISTS vault_sub_item (
    id          BLOB PRIMARY KEY NOT NULL,
    vault_id    BLOB NOT NULL,
    source_path TEXT NOT NULL,
    dest_path   TEXT,
    media_type  TEXT NOT NULL DEFAULT 'ANIME',
    media_id    TEXT,
    title       TEXT NOT NULL,
    raw_title   TEXT NOT NULL,
    season      TEXT,
    episode     TEXT,
    status      TEXT NOT NULL DEFAULT 'PROCESSING',
    total_bytes INTEGER NOT NULL DEFAULT 0,
    progress    REAL NOT NULL DEFAULT 0.0,
    speed_bps   INTEGER NOT NULL DEFAULT 0,
    eta_seconds INTEGER,
    error_msg   TEXT,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (vault_id) REFERENCES vault(id) ON DELETE CASCADE
);

-- 4. vault_metadata
CREATE TABLE IF NOT EXISTS vault_metadata (
    id             BLOB PRIMARY KEY NOT NULL,
    sub_item_id    BLOB NOT NULL UNIQUE,
    file_name      TEXT NOT NULL,
    file_path      TEXT NOT NULL,
    thumbnail_path TEXT,
    created_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (sub_item_id) REFERENCES vault_sub_item(id) ON DELETE CASCADE
);

-- 5. audio_tracks
CREATE TABLE IF NOT EXISTS audio_tracks (
    id          BLOB PRIMARY KEY NOT NULL,
    metadata_id BLOB NOT NULL,
    language    TEXT NOT NULL,
    title       TEXT NOT NULL,
    channels    TEXT NOT NULL,
    is_default  INTEGER NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (metadata_id) REFERENCES vault_metadata(id) ON DELETE CASCADE
);

-- 6. video_subtitles
CREATE TABLE IF NOT EXISTS video_subtitles (
    id          BLOB PRIMARY KEY NOT NULL,
    metadata_id BLOB NOT NULL,
    track       INTEGER NOT NULL,
    language    TEXT NOT NULL,
    title       TEXT NOT NULL,
    format      TEXT NOT NULL,
    file_path   TEXT NOT NULL,
    is_forced   INTEGER NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (metadata_id) REFERENCES vault_metadata(id) ON DELETE CASCADE
);

-- 7. video_chapters
CREATE TABLE IF NOT EXISTS video_chapters (
    id          BLOB PRIMARY KEY NOT NULL,
    metadata_id BLOB NOT NULL,
    chapter_id  INTEGER NOT NULL,
    title       TEXT NOT NULL,
    start_time  REAL NOT NULL,
    end_time    REAL NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (metadata_id) REFERENCES vault_metadata(id) ON DELETE CASCADE
);

-- 8. subtitle_fonts
CREATE TABLE IF NOT EXISTS subtitle_fonts (
    id          BLOB PRIMARY KEY NOT NULL,
    metadata_id BLOB NOT NULL,
    font_name   TEXT NOT NULL,
    file_path   TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (metadata_id) REFERENCES vault_metadata(id) ON DELETE CASCADE
);
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

| Controller  | Method | Path                          | Description                                                              |
| :---------- | :----- | :---------------------------- | :----------------------------------------------------------------------- |
| **User**    | `POST` | `/api/v1/user/login`          | Authenticate by username, provider, sandbox flag, and passcode           |
| **User**    | `POST` | `/api/v1/user/add`            | Validate user against upstream provider and upsert record                |
| **User**    | `POST` | `/api/v1/user/all`            | Return all registered users in database                                  |
| **User**    | `POST` | `/api/v1/user/one`            | Fetch single user by UUID                                                |
| **User**    | `POST` | `/api/v1/user/delete`         | Delete user by UUID (cascades to user's vault items)                     |
| **User**    | `POST` | `/api/v1/user/oauth/exchange` | Exchange OAuth authorization code + PKCE verifier for access token       |
| **Media**   | `POST` | `/api/v1/media/anime`         | Fetch user's anime list with pagination and filtering                    |
| **Media**   | `POST` | `/api/v1/media/manga`         | Fetch user's manga list with pagination and filtering                    |
| **Crawler** | `POST` | `/api/v1/crawler/search`      | Search external torrent/media crawlers and parse titles                  |
| **Vault**   | `POST` | `/api/v1/vault/add`           | Add download item to vault and dispatch to download engine               |
| **Vault**   | `GET`  | `/api/v1/vault/all`           | Server-Sent Events (SSE) streaming real-time download and process states |
| **Vault**   | `POST` | `/api/v1/vault/pause`         | Pause active direct or torrent download                                  |
| **Vault**   | `POST` | `/api/v1/vault/resume`        | Resume paused download                                                   |
| **Vault**   | `POST` | `/api/v1/vault/delete`        | Cancel/delete download task and delete files from disk                   |
| **Vault**   | `POST` | `/api/v1/vault/metadata`      | Batch fetch sub-items and relational metadata by vault IDs               |
| **Vault**   | `GET`  | `/api/v1/vault/stream/{*path}`| Range-compatible streaming for media, DASH manifests, and subtitles      |

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

The download management subsystem provides multi-backend file acquisition with automated lifecycle management:

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
              │ SQLite DB  │ │ SSE Stream │ (/vault/all)
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
  - Automatically resumes incomplete downloads (`PENDING` or `DOWNLOADING`) upon server startup and links them to the post-processor.
- **`TorrentDownloader` (`downloaders/torrent.rs`)**:
  - Wraps `librqbit::Session` and manages `ManagedTorrent` handles.
  - Handles `MAGNET` links and `TFILE` torrent files.
  - Updates progress bytes, speeds, and completion states.
- **`DirectDownloader` (`downloaders/direct.rs`)**:
  - Streams HTTP/HTTPS files directly into destination paths.
  - Inspects existing file size on disk and sends `Range: bytes={downloaded_bytes}-` for resumable downloads.
  - Computes rolling transfer speeds (`speed_bps`) and ETAs (`eta_seconds`).
  - Uses `tokio_util::sync::CancellationToken` for non-blocking pause/cancel operations.
- **Background Daemon (`downloaders/daemon.rs`)**:
  - Runs in a background Tokio task.
  - Polls engine stats every 2 seconds when downloads are active.
  - Flushes progress to SQLite via `vault::ActiveModel::update_progress_mut()`.
  - On download completion, transitions item status to `PROCESSING` and dispatches files to `MediaProcessor`.

---

## 10. Media Post-Processing & Streaming Subsystem (`src/streaming/`)

The media streaming subsystem handles the transformation of downloaded media files into web-streamable formats and extracts auxiliary assets.

```
       ┌────────────────────────┐
       │   Download Complete    │
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │     MediaProcessor     │
       │ (Folder/File Scanning) │
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │     VideoProcessor     │
       │ (FFmpeg / ffprobe Pipe)│
       └─────┬─────┬──────┬─────┘
             │     │      │
     ┌───────┘     │      └───────┐
     ▼             ▼              ▼
┌──────────┐ ┌───────────┐ ┌──────────────┐
│ DASH/MP4 │ │ Subtitles │ │ Font Attach- │
│ Manifest │ │ (ASS/SRT) │ │ ments & Tags │
└────┬─────┘ └─────┬─────┘ └──────┬───────┘
     └─────────────┼──────────────┘
                   │
                   ▼
      ┌─────────────────────────┐
      │  Vault Sub-Item & Meta  │ ──► Persist to SQLite
      └────────────┬────────────┘
                   │
                   ▼
      ┌─────────────────────────┐
      │ Monitoring Daemon (SSE) │ ──► Broadcast via AppEvent
      └─────────────────────────┘
```

### 10.1 Processing Pipeline

1. **Discovery (`MediaProcessor::process_folder`)**:
   - Recursively traverses `{dest_path}` with `WalkDir` while ignoring the encoded directory (`ENCODED_LOC`).
   - Disregards small files (< 1MB) to prevent processing ad-clips or samples.
   - Dispatches identified media to `VideoProcessor::post_process`.
2. **Analysis (`ffprobe`)**:
   - Parses stream layouts, video resolutions, frame rates, audio track languages, and subtitle streams using `rust_ffprobe` with string-to-numeric normalization.
3. **Extraction**:
   - **Audio Tracks**: Extracted and tagged with ISO languages and channel configurations.
   - **Subtitles**: Extracted into `subs/` as raw ASS, SRT, or WebVTT.
   - **Fonts**: Embedded attachments extracted into `fonts/` for accurate browser rendering of stylized ASS subtitles.
   - **Chapters**: Extracted with start/end millisecond timestamps.
   - **Thumbnails**: Keyframe extracted at regular interval for scrubber previews.
4. **Transcoding & Packaging**:
   - Transcodes video into web-compatible MP4 / DASH streaming formats.
   - Progress is throttled and reported via `StreamingEvent::Progress`.
5. **Progress Aggregation & Monitoring (`src/streaming/daemon.rs`)**:
   - Background monitoring task aggregates progress from individual sub-items into the parent `VaultItem`.
   - Emits real-time `StreamingEvent` notifications to `Sender<AppEvent>`.
6. **Range & Video Streaming (`src/controllers/vault_stream.rs`)**:
   - `GET /api/v1/vault/stream/{*path}` validates that the target file resides strictly inside the vault directory (`is_file_in_vault`).
   - Employs `tower_http::services::ServeFile` to deliver byte-range responses (`206 Partial Content`) for instant seeking in video players.

---

## 11. Shared State & Application Lifecycle (`src/app.rs`)

### 11.1 `AppContext.shared_store` Singletons

The following singletons are initialized and stored in `AppContext.shared_store`:

1. `reqwest::Client`: Shared HTTP client configured with connection pooling and custom headers.
2. `tokio::sync::broadcast::Sender<AppEvent>`: Global broadcast channel (capacity 128) streaming download and transcoding events to Server-Sent Event consumers.
3. `Arc<MediaProcessor>`: Post-processing coordinator handling directory scanning, file cleanup, and transcoding pipeline dispatch.
4. `Arc<DownloadManager>`: Global download orchestrator coordinating direct downloads and BitTorrent sessions.

### 11.2 Daemons & Startup Hooks

During `App::after_context()`:
- `start_daemon(&ctx)`: Launches the download polling task.
- `download_manager.auto_resume(&media_processor)`: Queries database for unfinished downloads and re-enqueues them.
- `start_monitoring(&ctx)`: Starts the media post-processing and aggregation daemon.

### 11.3 Graceful Shutdown (`on_shutdown`)

When a termination signal (`SIGINT` / `SIGTERM`) is received:
- Spawns a 10-second watchdog task that forces process termination if cleanup hangs.
- Iterates over all active download engines and cleanly shuts down active torrent sessions and DHT states.

---

## 12. Development & Testing Workflow

### 12.1 Essential Commands

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

### 12.2 Environment Variables

| Variable                | Description                                       | Default                                 |
| :---------------------- | :------------------------------------------------ | :-------------------------------------- |
| `PORT`                  | Server listening port                             | `5150`                                  |
| `BINDING`               | Server bind host interface                        | `localhost`                             |
| `DATABASE_URL`          | SQLite database URI                               | `sqlite://assets/main.sqlite?mode=rwc`  |
| `QUEUE_URL`             | SQLite queue URI                                  | `sqlite://assets/queue.sqlite?mode=rwc` |
| `VAULT_LOC`             | Root directory for downloaded vault files         | `vault`                                 |
| `ENCODED_LOC`           | Output directory for processed/transcoded files   | `encoded`                               |
| `MAL_CLIENT_ID`         | MyAnimeList API Client ID                         | (Optional for OAuth)                    |
| `MAL_CLIENT_SECRET`     | MyAnimeList API Client Secret                     | (Optional for OAuth)                    |
| `ANILIST_CLIENT_ID`     | AniList API Client ID                             | (Optional for OAuth)                    |
| `ANILIST_CLIENT_SECRET` | AniList API Client Secret                         | (Optional for OAuth)                    |
