# System Architecture & Codebase Context (`komorebi-server`)

## 1. Overview & Purpose

`komorebi-server` is a high-performance, unified REST API server written in Rust, built on the [Loco](https://loco.rs) framework. It serves as an aggregation, synchronization, and list management backend for anime and manga media items across multiple third-party providers (MyAnimeList and AniList).

The system abstracts differences in provider data models, score scales, formats, and pagination into a single normalized REST API. It also includes a crawler subsystem for scraping torrent/media metadata from external sites (e.g. Nyaa.si).

---

## 2. Technology Stack

| Layer         | Technology                  |
| ------------- | --------------------------- |
| Language      | Rust (Edition 2021)         |
| Framework     | [Loco](https://loco.rs) 1.1 |
| Web           | Axum 0.8                    |
| Database      | SQLite via Sea-ORM 2.0      |
| Async runtime | Tokio                       |
| HTTP client   | reqwest 0.13                |
| IDs           | UUID v7                     |
| Timestamps    | chrono                      |
| HTML parsing  | scraper 0.27                |
| Enum strings  | strum / strum_macros 0.28   |
| TS bindings   | ts-rs 12                    |

---

## 3. Codebase Map & Directory Layout

```
komorebi-server/
├── config/                    # Per-environment YAML configs (development / production / test)
├── docs/
│   ├── CONTEXT.md             # This file — architecture & codebase reference
│   ├── openapi.yaml           # OpenAPI 3.1 specification
│   ├── schema.sql             # SQLite table DDL reference
│   └── *.schema.json          # JSON schemas for upstream provider responses (MAL, AniList)
├── migration/                 # Sea-ORM migrations
├── src/
│   ├── lib.rs                 # Library crate root
│   ├── bin/
│   │   ├── main.rs            # CLI entrypoint (komorebi_server-cli)
│   │   └── tool.rs            # Secondary bin target
│   ├── app.rs                 # Loco Hooks impl — route & worker registration, shared state
│   ├── adapters/              # Upstream provider client abstractions & DTO adapters
│   │   ├── mod.rs             # MediaClient trait, MediaClientParams, MediaProvider::new_client()
│   │   ├── mal_client.rs      # MyAnimeList REST API client implementation
│   │   ├── mal_models.rs      # MAL DTO models & normalization logic
│   │   ├── anilist_client.rs  # AniList GraphQL API client implementation
│   │   └── anilist_models.rs  # AniList GraphQL DTO models & normalization logic
│   ├── controllers/
│   │   ├── mod.rs             # success() / fail() response envelope helpers
│   │   ├── media_controller.rs    # POST /media/anime, /media/manga
│   │   ├── user_controller.rs     # POST /user/login|add|all|one|delete|oauth/exchange
│   │   └── crawler_controller.rs  # POST /crawler/search (WIP)
│   ├── models/
│   │   ├── _entities/         # Generated Sea-ORM entities — do not hand-edit
│   │   ├── mod.rs             # Module exports
│   │   ├── media.rs           # Core domain types: Media, ListEntry, MediaEntry, enums
│   │   ├── users.rs           # User model, ActiveModelBehavior, DB operations
│   │   └── crawler.rs         # CrawlerConfig, CrawlerResult, ParsedTitle types
│   ├── crawlers/              # Web scraping / title parsing subsystem
│   │   ├── mod.rs             # Crawler & TitleParser traits
│   │   ├── html_crawler.rs    # HTML scraping via scraper crate
│   │   ├── json_crawler.rs    # JSON-based crawler
│   │   ├── config_parser.rs   # YAML crawler config loader
│   │   └── title_parser.rs    # Torrent title parsing (Anitomy-style)
│   ├── core/
│   │   ├── mod.rs             # Core module exports
│   │   └── constants.rs       # App-wide constants
│   ├── dtos/
│   │   ├── mod.rs             # DTO module exports
│   │   └── common.rs          # Page<T> paginated response, ApiError — ts-rs exported to frontend
│   ├── initializers/
│   │   ├── mod.rs             # Initializer module
│   │   └── client.rs          # reqwest::Client builder (injected into ctx.shared_store)
│   ├── workers/
│   │   ├── mod.rs             # Worker module exports
│   │   └── downloader.rs      # DownloadWorker — async Loco background worker
│   ├── tasks/
│   │   └── mod.rs             # CLI task stubs (tasks-inject marker)
│   ├── views/
│   │   └── mod.rs             # View module (currently empty)
│   └── data/
│       └── mod.rs             # Data module (currently empty)
└── tests/                     # Integration & unit tests
```

---

## 4. Domain Data Models

### 4.1 Media Enums (`src/models/media.rs`)

- **`MediaProvider`**: Upstream source (`MAL`, `ANILIST`). Sea-ORM `DeriveActiveEnum` — persisted as UPPERCASE string.
- **`MediaType`**: Kind of media (`Anime`, `Manga`, `Novel`, `Other(String)`). Case-insensitive `FromStr` via strum.
- **`MediaFormat`**: Unified release medium superset across MAL and AniList (`Unknown`, `Tv`, `TvShort`, `Movie`, `Special`, `Ova`, `Ona`, `Music`, `Manga`, `Novel`, `OneShot`, `Doujinshi`, `Manhwa`, `Manhua`, `Oel`).
- **`ReleaseStatus`**: Airing/publishing status (`Unknown`, `Releasing`, `Finished`, `NotYetReleased`, `Cancelled`, `Hiatus`).
- **`ListStatus`**: Personal watch/read status (`Current`, `Planning`, `Completed`, `Dropped`, `Paused`, `Repeating`).
- **`NsfwLevel`**: Maturity rating (`Safe`, `Gray`, `Nsfw`). MAL has three tiers; AniList maps `isAdult` bool to `Safe` / `Nsfw`.

### 4.2 Value Objects (`src/models/media.rs`)

- **`MediaTitle`**: Multi-language title — `romanized`, `english`, `native`, `user_preferred`.
- **`CoverImage`**: Cover art URLs — `extra_large` (AniList only), `large`, `medium`, `color` (AniList only).

### 4.3 Core Entities

- **`Media`**: Provider-agnostic metadata for an anime or manga title. Fields: `id` (UUID v7), `provider_id`, `provider`, `media_type`, `format`, `release_status`, `title`, `cover`, `synopsis`, `mean_score`, `popularity`, `episodes`, `duration`, `chapters`, `volumes`, `genres`, `nsfw`.
- **`ListEntry`**: User-specific tracking — `status`, `score` (normalized 0.0–10.0), `progress`, `progress_volumes`, `is_repeating`, `repeat_count`, `tags`, `notes`, `updated_at`.
- **`MediaEntry`**: Pair of `(Media, ListEntry)` — one per list item returned by the API.
- **`PaginatedResponse`**: `Vec<MediaEntry>` + `Paging` (`next_cursor`, `prev_cursor`, `has_next`).
- **`User`** / **`users::Model`** (`src/models/users.rs`): Sea-ORM entity. Fields: `id` (UUID v7), `username`, `avatar_url`, `provider`, `is_sandbox`, `passcode`, `access_token`, `created_at`, `updated_at`. Persisted via generated `_entities/users`.

### 4.4 Crawler Types (`src/models/crawler.rs`)

- **`CrawlerConfig`**: Scraper config — `id`, `name`, `base_url`, CSS selectors, `is_active`, `category`. Falls back to a built-in Nyaa.si config.
- **`CrawlerResult`**: Single scraped item — `title`, `link`, `source`, `popularity`, `size`.
- **`ParsedTitle`**: Structured fields parsed from a raw torrent filename (episode, season, release_group, resolution, etc.).

---

## 5. Persistence Layer

### 5.1 Database (Sea-ORM + SQLite)

Sea-ORM manages the schema via migrations (`migration/`). The `_entities/` directory is fully generated — do not hand-edit it.

**`users` table:**

```sql
CREATE TABLE IF NOT EXISTS users (
    id           BLOB    PRIMARY KEY NOT NULL,
    username     TEXT    NOT NULL,
    avatar_url   TEXT,
    provider     TEXT    NOT NULL,
    is_sandbox   BOOLEAN NOT NULL DEFAULT 1,
    access_token TEXT,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    updated_at   INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    UNIQUE(username, provider)
);
```

### 5.2 User Model Operations (`src/models/users.rs`)

| Method                                      | Description                                                                  |
| ------------------------------------------- | ---------------------------------------------------------------------------- |
| `save_user`                                 | UPSERT on `(username, provider, is_sandbox)` conflict. Returns saved `User`. |
| `get_all_users`                             | Returns all users.                                                           |
| `find_by_id`                                | Finds user by UUID.                                                          |
| `find_by_username_and_provider_and_sandbox` | Used for login lookup.                                                       |
| `delete_user`                               | Deletes by UUID.                                                             |
| `verify_passcode`                           | Argon2 hash comparison; falls back to plain-text for unhashed passcodes.     |

### 5.3 `ActiveModelBehavior` (`src/models/users.rs`)

The `before_save` hook:

- Auto-assigns `Uuid::now_v7()` on insert if ID is nil.
- Derives `is_sandbox = access_token.is_none_or_empty()`.
- Sets `created_at` on insert; always updates `updated_at`.

---

## 6. HTTP API & Controller Architecture

### 6.1 Route Table

All routes are under `/api/v1` (set in `src/app.rs`).

| Method | Path                          | Handler                                  |
| ------ | ----------------------------- | ---------------------------------------- |
| `POST` | `/api/v1/user/login`          | `user_controller::login`                 |
| `POST` | `/api/v1/user/add`            | `user_controller::save_user`             |
| `POST` | `/api/v1/user/all`            | `user_controller::get_all_users`         |
| `POST` | `/api/v1/user/one`            | `user_controller::get_user_by_id`        |
| `POST` | `/api/v1/user/delete`         | `user_controller::delete_user_by_id`     |
| `POST` | `/api/v1/user/oauth/exchange` | `user_controller::exchange_oauth_token`  |
| `POST` | `/api/v1/media/anime`         | `media_controller::get_user_anime`       |
| `POST` | `/api/v1/media/manga`         | `media_controller::get_user_manga`       |
| `POST` | `/api/v1/crawler/search`      | `crawler_controller::search_media` (WIP) |

Plus Loco's default routes (`GET /_health`, `GET /_ping`, etc.).

### 6.2 Response Envelopes (`src/controllers/mod.rs`)

```json
// Success (200 OK)
{ "success": true, "data": { ... } }

// Failure (4xx / 5xx)
{ "success": false, "error": "ERROR_CODE", "description": "Optional detail" }
```

### 6.3 User Controller Highlights

- **`login`**: Finds user by `(username, provider, is_sandbox)`, then verifies passcode via Argon2.
- **`save_user`**: Validates against the provider (OAuth token or list probe), then upserts to DB.
- **`exchange_oauth_token`**: Proxies code + PKCE verifier to the relevant provider adapter.
- **`validate_user`** (internal): Uses `validate_new_user()` when token is present; otherwise probes the anime/manga list.

### 6.4 Media Controller

Both handlers look up the user by `params.user_id`, then dispatch to the correct adapter:

- **`get_user_anime`**: Returns `PaginatedResponse` of anime list entries.
- **`get_user_manga`**: Returns `PaginatedResponse` of manga list entries.

---

## 7. Provider Adapter Layer (`src/adapters/`)

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

`MediaProvider::new_client()` dispatches to `MalClient` or `AniListClient` based on the user's provider.

### 7.2 `MediaClientParams`

```rust
pub struct MediaClientParams {
    pub user_id: Uuid,
    pub status: Option<String>,
    pub sort:   Option<String>,
    pub limit:  Option<i32>,   // default: 50
    pub offset: Option<i32>,   // default: 0
}
```

### 7.3 `MalClient`

- REST endpoints: `/v2/users/{username}/animelist`, `/v2/users/{username}/mangalist`.
- Score scale: 0–10 (no normalization needed).
- Pagination: cursor URLs from MAL's `paging.next` field.

### 7.4 `AniListClient`

- GraphQL endpoint: `https://graphql.anilist.co`.
- Score scale: 0–100 → normalized to 0.0–10.0.
- Pagination: page-based (`page`, `perPage`).

---

## 8. Crawler Subsystem (`src/crawlers/`)

Scrapes external sites (primarily Nyaa.si) to find download links for media titles.

### 8.1 Traits

```rust
pub trait Crawler {
    fn can_crawl(content: &str) -> bool;
    async fn crawl(content: &str, config: &CrawlerConfig) -> Vec<CrawlerResult>;
}

pub trait TitleParser {
    fn can_parse(raw_title: &str) -> bool;
    async fn parse(raw_title: &str) -> ParsedTitle;
}
```

### 8.2 Implementations

- **`HtmlCrawler`**: CSS-selector-based extraction via `scraper`.
- **`JsonCrawler`**: Field-mapped JSON API parsing.
- **`config_parser`**: YAML crawler config loader.
- **`title_parser`**: Anitomy-style torrent filename → `ParsedTitle`.

---

## 9. Shared Infrastructure

### 9.1 reqwest Client (`src/initializers/client.rs`)

Built once at startup; injected into `ctx.shared_store` via `App::after_context()`. Retrieved in handlers with `ctx.shared_store.get::<Client>()`.

### 9.2 Background Workers (`src/workers/downloader.rs`)

`DownloadWorker` is a Loco background worker registered with the job queue via `App::connect_workers()`.

### 9.3 TypeScript Bindings (`src/dtos/common.rs`)

`ts-rs` derives `Page<T>` and `ApiError` TypeScript types and exports them to `frontend/src/bindings/` at build time.

---

## 10. Development Workflow

```sh
# Apply database migrations
cargo loco db migrate

# Start the development server (http://localhost:5150)
cargo loco start

# List all registered routes
cargo loco routes

# Check environment health
cargo loco doctor

# Run all tests
cargo test
```

### Generators

```sh
cargo loco generate model <name> [fields...]
cargo loco generate scaffold <name>
cargo loco generate controller <name>
cargo loco generate worker <name>
cargo loco generate task <name>
cargo loco generate migration <name>
```

---

## 2. Technology Stack

- **Language & Runtime**: Rust (Edition 2024 / 2021) with `tokio` async runtime.
- **Web Framework**: `axum` 0.8 with `tower-http` middleware for HTTP tracing.
- **Database & Persistence**: SQLite managed via `sqlx` 0.8 with asynchronous connection pooling.
- **Serialization & Data Models**: `serde`, `serde_json`, `strum` (for enum string conversions), `uuid` (v7 IDs), `chrono` (timestamps).
- **HTTP Client**: `reqwest` for executing upstream client requests to MyAnimeList REST API and AniList GraphQL API.
- **Logging & Diagnostics**: `tracing` and `tracing-subscriber`.

---

## 3. Codebase Map & Directory Layout

```
komorebi-server/
├── assets/
│   └── schema.sql              # SQLite table creation script & triggers
├── docs/
│   └── openapi.yaml            # OpenAPI 3.1 specification for the server
├── src/
│   ├── main.rs                 # Server entrypoint (initialization & Axum serve)
│   ├── lib.rs                  # Library crate root
│   ├── adapters/               # Upstream provider client abstractions & DTO adapters
│   │   ├── mod.rs              # MediaClient trait & shared query parameters
│   │   ├── mal_client.rs   # MyAnimeList REST API client implementation
│   │   ├── mal_models.rs   # MAL DTO models & conversion logic
│   │   ├── anilist_client.rs # AniList GraphQL API client implementation
│   │   └── anilist_models.rs # AniList GraphQL DTO models & conversion logic
│   ├── config.rs               # Environment variable config parsing
│   ├── db/                     # Database access layer
│   │   ├── mod.rs              # DbState pool wrapper & database connections
│   │   └── user_repo.rs        # Repository CRUD operations for User entities
│   ├── handlers/               # HTTP route handlers & responses
│   │   ├── mod.rs              # Route setup (make_routes), standard response envelopes
│   │   └── media_handler.rs    # /media/anime & /media/manga route implementations
│   ├── models/                 # Domain entities & value objects
│   │   ├── mod.rs              # Models module export
│   │   ├── media.rs            # Core Media, ListEntry, MediaEntry, & enum types
│   │   └── user.rs             # User domain entity struct & constructors
│   ├── services/               # Application service layer
│   │   ├── mod.rs              # Services module export
│   │   └── media_service.rs    # User list fetching and provider orchestration
│   ├── startup.rs              # Axum app builder, listener setup, signal handlers
│   ├── state.rs                # AppState definition & database/app directory paths
│   └── telemetry.rs            # Tracing logger setup & file appender
└── tests/                      # Integration & unit test suites
    ├── user_model_test.rs      # Unit tests for User sandbox calculation
    ├── user_repo_test.rs       # In-memory SQLite tests for UserRepo CRUD/Upsert
    ├── mal_client_test.rs      # Parsing & mapping tests for MAL responses
    └── anilist_client_test.rs  # Parsing & mapping tests for AniList responses
```

---

## 4. Domain Data Models

### 4.1 Media Enums (`src/models/media.rs`)

- **`MediaProvider`**: Supported third-party sources (`MAL`, `ANILIST`). Serialized in UPPERCASE.
- **`MediaType`**: Type of media (`Anime`, `Manga`).
- **`MediaFormat`**: Supersed release medium (`Unknown`, `Tv`, `TvShort`, `Movie`, `Special`, `Ova`, `Ona`, `Music`, `Manga`, `Novel`, `OneShot`, `Doujinshi`, `Manhwa`, `Manhua`, `Oel`).
- **`ReleaseStatus`**: Airing/publishing status (`Unknown`, `Releasing`, `Finished`, `NotYetReleased`, `Cancelled`, `Hiatus`).
- **`ListStatus`**: Personal watch/read status (`Current`, `Planning`, `Completed`, `Dropped`, `Paused`, `Repeating`).
- **`NsfwLevel`**: Maturity rating (`Safe`, `Gray`, `Nsfw`).

### 4.2 Core Entities (`src/models/media.rs` & `src/models/user.rs`)

- **`User`**: Represents a user account linked to a provider (`id`, `username`, `avatar_url`, `provider`, `is_sandbox`, `access_token` [skipped in JSON], `created_at`, `updated_at`).
- **`Media`**: Provider-agnostic metadata representing an anime or manga title (`id`, `provider_id`, `provider`, `media_type`, `format`, `release_status`, `title`, `cover`, `synopsis`, `mean_score`, `popularity`, `episodes`, `duration`, `chapters`, `volumes`, `genres`, `nsfw`).
- **`ListEntry`**: User-specific tracking metrics (`status`, `score`, `progress`, `progress_volumes`, `is_repeating`, `repeat_count`, `tags`, `notes`, `updated_at`).
- **`MediaEntry`**: Pair of `(Media, ListEntry)` returned as list items.
- **`PaginatedResponse`**: Wrapper holding a `Vec<MediaEntry>` and `Paging` metadata (`next_cursor`, `prev_cursor`, `has_next`).

---

## 5. Persistence Layer & Database Schema (`docs/schema.sql`, `src/db/`)

### 5.1 SQLite Schema (`users` table)

```sql
CREATE TABLE IF NOT EXISTS users (
    id BLOB PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    avatar_url TEXT,
    provider TEXT NOT NULL,
    is_sandbox BOOLEAN NOT NULL DEFAULT 1,
    access_token TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    UNIQUE(username, provider)
);
```

### 5.2 Repository Operations (`src/db/user_repo.rs`)

- **`save_user`**: Performs an UPSERT on `(username, provider)` conflict. Dynamically evaluates `is_sandbox = access_token.is_none()`. Returns the updated `User`.
- **`fetch_user_by_id`**: Retrieves optional `User` by UUID v7 key.
- **`fetch_user_by_username`**: Retrieves optional `User` by `(username, provider)`.
- **`delete_user`**: Deletes record by UUID v7 key.

---

## 6. HTTP API & Handler Architecture

### 6.1 Route Registration (`src/handlers/mod.rs`)

- `POST /` -> `health_check_bad` (Returns 400 Bad Request redirecting clients to `/api/v1`).
- `POST /api/v1` -> `health_check` (Returns uptime, version `1.0.0`, and base URL).
- `POST /api/v1/health` -> `health_check`.
- `POST /api/v1/media/anime` -> `get_user_anime_list`.
- `POST /api/v1/media/manga` -> `get_user_manga_list`.
- `POST /api/v1/user/add` -> `save_user`.
- `POST /api/v1/user/all` -> `get_all_users`.
- `POST /api/v1/user/one` -> `get_user_by_id`.
- `POST /api/v1/user/delete` -> `delete_user`.

### 6.2 Standardized Response Envelopes

All responses adhere to strict JSON envelopes:

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
    "error": {
      "code": "FETCH_ANIME_FAILED",
      "msg": "Detailed error message"
    }
  }
  ```

---

## 7. Upstream Provider Abstraction (`src/handlers/clients/`)

Provider clients implement the async trait `MediaClient`:

```rust
#[async_trait]
pub trait MediaClient {
    async fn get_anime_list(db: DbState, params: &MedialClientParams) -> Result<PaginatedResponse, Box<dyn std::error::Error>>;
    async fn get_manga_list(db: DbState, params: &MedialClientParams) -> Result<PaginatedResponse, Box<dyn std::error::Error>>;
}
```

- **`MalClient`**: Fetches user list from MyAnimeList REST API endpoints (`/v2/users/{username}/animelist`, `/v2/users/{username}/mangalist`). Normalizes 0–10 score scale and cursors.
- **`AniListClient`**: Queries AniList GraphQL API (`https://graphql.anilist.co`). Normalizes 0–100 score scale to 0.0–10.0 float and handles page-based pagination.

---

## 8. Development & Testing Workflow

### 8.1 Running Tests

```bash
cargo test
```

The test suite includes:

1. `user_model_test.rs`: Unit testing for `is_sandbox` boolean derivation.
2. `user_repo_test.rs`: Integration tests utilizing SQLite in-memory databases (`sqlite::memory:`).
3. `mal_client_test.rs` & `anilist_client_test.rs`: DTO deserialization and model harmonization verification.

### 8.2 Building & Running

```bash
cargo build
cargo run
```
