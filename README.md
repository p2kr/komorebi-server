# komorebi-server

A high-performance, unified media backend server written in Rust — built on the [Loco](https://loco.rs) framework. It aggregates and normalizes anime and manga list data across multiple third-party providers (MyAnimeList, AniList), provides a configurable web crawler subsystem, and features a multi-backend download manager with real-time WebSocket progress streaming.

## Features

- **Multi-provider support** — MyAnimeList (REST) and AniList (GraphQL) behind a unified, normalized interface
- **Normalized data models** — scores (0.0–10.0 scale), formats, statuses, and pagination unified regardless of upstream provider
- **User management & Passcode auth** — register, look up, and delete users linked to provider accounts with optional Argon2 passcode protection and sandbox mode
- **OAuth token exchange** — exchange OAuth authorization codes + PKCE verifiers for provider access tokens
- **Paginated list fetching** — anime and manga lists with filtering, sorting, and cursor/offset pagination
- **Multi-backend Download Engine** — concurrent downloading for Direct HTTP/HTTPS (with HTTP `Range` resumption) and BitTorrent / Magnet links via `librqbit`
- **Real-time WebSockets** — live download progress, speed, ETA, and state streaming over `/api/v1/vault/ws`
- **Isolated Vault Storage** — per-item UUID v7 isolated folders (`{VAULT_LOC}/{vault_id}/`), download lifecycle control (add, pause, resume, delete), and DB tracking
- **Crawler subsystem** — scrapes torrent and media sites (e.g. Nyaa.si) using configurable YAML/CSS selector rules and Anitomy-based title parsing
- **TypeScript bindings** — `ts-rs` automatically emits TypeScript types from Rust DTOs for frontend consumption
- **Background workers & Daemons** — Loco background worker queue alongside a dedicated download polling daemon

## Tech Stack

| Layer             | Technology                       |
| ----------------- | -------------------------------- |
| Language          | Rust (Edition 2024)              |
| Framework         | [Loco](https://loco.rs) 1.1      |
| Web & WebSockets  | Axum 0.8 (with `ws` feature)     |
| Database          | SQLite via Sea-ORM 2.0           |
| Async runtime     | Tokio                            |
| HTTP client       | reqwest 0.13                     |
| BitTorrent engine | librqbit 9.0                     |
| IDs               | UUID v7                          |
| Timestamps        | chrono                           |
| HTML parsing      | scraper 0.27                     |
| Title parsing     | anitomy-rs                       |
| Enum strings      | strum / strum_macros 0.28        |
| TS bindings       | ts-rs 12                         |

## Project Layout

```
komorebi-server/
├── assets/                  # Default SQLite databases, crawler YAML configs, DHT cache
├── config/                  # Per-environment YAML configs (development / production / test)
├── docs/
│   ├── CONTEXT.md           # Architecture & codebase reference
│   ├── openapi.yaml         # OpenAPI 3.1 specification
│   ├── schema.sql           # SQLite table DDL reference
│   └── *.schema.json        # Upstream provider response schemas (MAL, AniList)
├── migration/               # Sea-ORM migrations (users, vaults)
├── src/
│   ├── app.rs               # Loco Hooks impl — route, worker, and shared state wiring hub
│   ├── adapters/            # Upstream provider client abstractions & DTO adapters
│   │   ├── mod.rs           # MediaClient trait + MediaClientParams + provider dispatch
│   │   ├── mal_client.rs    # MyAnimeList REST client
│   │   ├── mal_models.rs    # MAL DTO models & conversions
│   │   ├── anilist_client.rs  # AniList GraphQL client
│   │   └── anilist_models.rs  # AniList DTO models & conversions
│   ├── controllers/         # HTTP handlers grouped into Routes
│   │   ├── mod.rs           # Shared response envelopes (success / fail)
│   │   ├── user_controller.rs     # /api/v1/user routes
│   │   ├── media_controller.rs    # /api/v1/media routes
│   │   ├── crawler_controller.rs  # /api/v1/crawler routes
│   │   └── vault_controller.rs    # /api/v1/vault routes & WebSocket handler
│   ├── models/              # Domain entities & Sea-ORM models
│   │   ├── _entities/       # Generated Sea-ORM entities (do not edit)
│   │   ├── media.rs         # Core domain types (Media, ListEntry, MediaEntry, enums)
│   │   ├── users.rs         # User model, ActiveModelBehavior, DB operations
│   │   ├── crawler.rs       # CrawlerConfig, CrawlerResult, ParsedTitle
│   │   └── vault.rs         # VaultItem model, VaultDownloadType, VaultItemStatus
│   ├── crawlers/            # Web scraping & title parsing subsystem
│   │   ├── crawler_engine.rs      # Concurrent crawler orchestrator
│   │   ├── html_crawler.rs        # CSS-selector HTML scraper
│   │   ├── json_crawler.rs        # JSON API scraper
│   │   ├── config_parser.rs       # YAML crawler config loader
│   │   └── anitomy_title_parser.rs # Torrent title parser (Anitomy-rs)
│   ├── downloaders/         # Multi-backend download management
│   │   ├── mod.rs           # DownloadEngine trait
│   │   ├── manager.rs       # DownloadManager singleton
│   │   ├── direct.rs        # Direct HTTP downloader (Range resumption)
│   │   ├── torrent.rs       # Torrent & magnet downloader (librqbit)
│   │   └── daemon.rs        # Polling daemon & WebSocket broadcast pump
│   ├── core/                # App-wide constants, path resolvers, reqwest client & tracker fetcher, ResultExt
│   ├── dtos/                # TypeScript-exported DTO types (ts-rs)
│   └── workers/
│       └── downloader.rs    # Background download worker
└── tests/                   # Request, model, crawler, and adapter integration tests
```

## API Routes

All application routes are prefixed with `/api/v1`.

### User

| Method | Path                         | Description                                                    |
| ------ | ---------------------------- | -------------------------------------------------------------- |
| `POST` | `/api/v1/user/login`          | Authenticate a user by username + provider + optional passcode |
| `POST` | `/api/v1/user/add`            | Register / upsert a user (validates against provider)          |
| `POST` | `/api/v1/user/all`            | List all registered users                                      |
| `POST` | `/api/v1/user/one`            | Get a user by UUID                                             |
| `POST` | `/api/v1/user/delete`         | Delete a user by UUID                                          |
| `POST` | `/api/v1/user/oauth/exchange` | Exchange an OAuth code + PKCE verifier for an access token     |

### Media

| Method | Path                  | Description                                          |
| ------ | --------------------- | ---------------------------------------------------- |
| `POST` | `/api/v1/media/anime` | Fetch a user's anime list from their linked provider |
| `POST` | `/api/v1/media/manga` | Fetch a user's manga list from their linked provider |

### Crawler

| Method | Path                    | Description                                                   |
| ------ | ----------------------- | ------------------------------------------------------------- |
| `POST` | `/api/v1/crawler/search`| Search configured crawlers for media titles and parsed torrents |

### Vault (Downloads)

| Method | Path                   | Description                                                      |
| ------ | ---------------------- | ---------------------------------------------------------------- |
| `POST` | `/api/v1/vault/add`    | Add a download item to the vault (direct URL or magnet/torrent)  |
| `POST` | `/api/v1/vault/one`    | Get single vault item metadata and progress by UUID              |
| `POST` | `/api/v1/vault/all`    | List all vault items in database                                 |
| `POST` | `/api/v1/vault/pause`  | Pause an active direct or torrent download                       |
| `POST` | `/api/v1/vault/resume` | Resume a paused download                                         |
| `POST` | `/api/v1/vault/delete` | Delete / cancel a download and remove files from disk            |
| `GET`  | `/api/v1/vault/ws`     | WebSocket upgrade endpoint for live progress update stream       |

### Response Envelopes

All JSON API endpoints adhere to a uniform response envelope:

```json
// Success (200 OK)
{
  "success": true,
  "data": { ... }
}

// Failure (4xx / 5xx)
{
  "success": false,
  "error": "ERROR_CODE",
  "description": "Optional human-readable error description"
}
```

## Quick Start

```sh
# Apply database migrations
cargo loco db migrate

# Start the development server (defaults to http://localhost:5150)
cargo loco start
```

## Environment Variables

Configure provider credentials and storage settings in `.env` or via `config/*.yaml`:

| Variable                | Description                                                | Default              |
| ----------------------- | ---------------------------------------------------------- | -------------------- |
| `PORT`                  | Server port                                                | `5150`               |
| `BINDING`               | Server bind address                                        | `localhost`          |
| `DATABASE_URL`          | SQLite database URI                                        | `sqlite://assets/main.sqlite?mode=rwc` |
| `QUEUE_URL`             | SQLite job queue URI                                       | `sqlite://assets/queue.sqlite?mode=rwc` |
| `VAULT_LOC`             | Storage root directory for downloaded vault files          | `vault`              |
| `MAL_CLIENT_ID`         | MyAnimeList API client ID                                  |                      |
| `MAL_CLIENT_SECRET`     | MyAnimeList API client secret                              |                      |
| `ANILIST_CLIENT_ID`     | AniList OAuth client ID                                    |                      |
| `ANILIST_CLIENT_SECRET` | AniList OAuth client secret                                |                      |

## Development & Testing

```sh
# Run all unit and integration tests
cargo test

# List all registered routes
cargo loco routes

# Check environment health
cargo loco doctor

# Apply database migrations
cargo loco db migrate
```

## Resources

- [Loco Framework Documentation](https://loco.rs/docs)
- [OpenAPI Specification](docs/openapi.yaml)
- [Architecture & Codebase Reference](docs/CONTEXT.md)
- [SQLite Schema Reference](docs/schema.sql)
