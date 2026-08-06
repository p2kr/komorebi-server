# System Architecture & Codebase Context (`komorebi-server`)

## 1. Overview & Purpose

`komorebi-server` is a high-performance, unified REST API server written in Rust. It serves as an aggregation, synchronization, and list management backend for anime and manga media items across multiple third-party providers (such as MyAnimeList and AniList).

The system abstracts differences in provider data models, score scales, formats, and pagination methods into a single normalized REST API interface.

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

## 5. Persistence Layer & Database Schema (`assets/schema.sql`, `src/db/`)

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
