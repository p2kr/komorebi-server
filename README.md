# komorebi-server

A high-performance, unified media backend server written in Rust — built on the [Loco](https://loco.rs) framework. It aggregates and normalizes anime and manga list data across multiple third-party providers (MyAnimeList, AniList), provides a configurable web crawler subsystem, features a multi-backend download manager, and includes a complete media processing and video streaming pipeline with real-time Server-Sent Events (SSE).

## Features

- **Multi-provider support** — MyAnimeList (REST) and AniList (GraphQL) behind a unified, normalized interface
- **Normalized data models** — scores (0.0–10.0 scale), formats, statuses, and pagination unified regardless of upstream provider
- **User management & Passcode auth** — register, look up, and delete users linked to provider accounts with optional Argon2 passcode protection and sandbox mode
- **OAuth token exchange** — exchange OAuth authorization codes + PKCE verifiers for provider access tokens
- **Paginated list fetching** — anime and manga lists with filtering, sorting, and cursor/offset pagination
- **Multi-backend Download Engine** — concurrent downloading for Direct HTTP/HTTPS (with HTTP `Range` resumption) and BitTorrent / Magnet links via `librqbit`
- **Real-time Server-Sent Events (SSE)** — live download progress, transcoding state, speeds, and ETAs streamed over `/api/v1/vault/all`
- **Isolated Vault Storage** — per-item UUID v7 isolated folders (`{VAULT_LOC}/{vault_id}/`), download lifecycle control (add, pause, resume, delete), and DB tracking
- **Media Post-Processing & Transcoding** — automated FFmpeg transcoding, thumbnail generation, and stream extraction for audio tracks, embedded subtitles (ASS/SRT/VTT), fonts, and chapters
- **Sub-Item & Metadata Hierarchy** — automatic scanning of downloaded folders into distinct sub-items (`vault_sub_item`) with detailed metadata relations
- **Video & File Streaming** — direct range-compatible streaming of media files, DASH manifests, fonts, and extracted subtitles via `/api/v1/vault/stream/{*path}`
- **Crawler subsystem** — scrapes torrent and media sites (e.g. Nyaa.si) using configurable YAML/CSS selector rules and Anitomy-based title parsing
- **TypeScript bindings** — `ts-rs` automatically emits TypeScript types from Rust DTOs for frontend consumption
- **Background workers & Daemons** — Loco background worker queue, download polling daemon, and background media monitoring daemon

## Tech Stack

| Layer             | Technology                                      |
| ----------------- | ----------------------------------------------- |
| Language          | Rust (Edition 2024)                             |
| Framework         | [Loco](https://loco.rs) 1.1                     |
| Web & Streaming   | Axum 0.8 (SSE, Range streaming, ServeFile)      |
| Database          | SQLite via Sea-ORM 2.0                          |
| Async runtime     | Tokio                                           |
| HTTP client       | reqwest 0.13                                    |
| BitTorrent engine | librqbit 9.0                                    |
| Media Processing  | FFmpeg, FFprobe (`rust_ffmpeg`, `rust_ffprobe`) |
| IDs               | UUID v7                                         |
| Timestamps        | chrono                                          |
| HTML parsing      | scraper 0.27                                    |
| Title parsing     | anitomy-rs                                      |
| Caching           | cached                                          |
| Enum strings      | strum / strum_macros 0.28                       |
| TS bindings       | ts-rs 12                                        |

## Project Layout

```
komorebi-server/
├── assets/                  # Default SQLite databases, crawler YAML configs, DHT cache
├── config/                  # Per-environment YAML configs (development / production / test)
├── docs/
│   ├── CONTEXT.md           # Architecture & codebase reference
│   ├── openapi.yaml         # OpenAPI 3.1 specification
│   ├── schema.sql           # SQLite table DDL reference (8 tables)
│   └── *.schema.json        # Upstream provider response schemas (MAL, AniList)
├── migration/               # Sea-ORM migrations & initial SQL schema
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
│   │   ├── vault_controller.rs    # /api/v1/vault lifecycle & SSE routes
│   │   └── vault_stream.rs        # /api/v1/vault metadata & range streaming handlers
│   ├── streaming/           # Media post-processing & video streaming subsystem
│   │   ├── mod.rs           # PostProcessor trait & StreamingEvent enum
│   │   ├── processor.rs     # MediaProcessor lifecycle & file resolver
│   │   ├── video.rs         # VideoProcessor (FFmpeg transcoding, font/sub/audio extraction)
│   │   └── daemon.rs        # Background post-processing monitoring daemon
│   ├── models/              # Sea-ORM active models & entities
│   │   ├── _entities/       # Generated Sea-ORM entities (do not edit)
│   │   ├── users.rs         # User model, ActiveModelBehavior, DB operations
│   │   ├── vault.rs         # VaultItem model, VaultDownloadType, VaultStatus
│   │   ├── vault_sub_item.rs# VaultSubItem model (individual media files in folder)
│   │   ├── vault_metadata.rs# VaultMetadata model (probed media info & thumbnails)
│   │   ├── audio_tracks.rs  # Extracted audio tracks per sub-item
│   │   ├── video_subtitles.rs # Extracted subtitle tracks per sub-item
│   │   ├── video_chapters.rs  # Extracted chapter markers per sub-item
│   │   └── subtitle_fonts.rs  # Extracted embedded fonts per sub-item
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
│   │   └── daemon.rs        # Polling daemon & stats persistence
│   ├── core/                # App-wide constants, path resolvers, reqwest client & tracker fetcher, ResultExt
│   ├── dtos/                # Domain types & TypeScript-exported DTOs (ts-rs)
│   │   ├── crawler.rs       # CrawlerConfig, CrawlerResult, ParsedTitle
│   │   ├── enums.rs         # VaultStatus, MediaType, MediaFormat, ListStatus, etc.
│   │   ├── events.rs        # AppEvent & SSE serialization
│   │   ├── media.rs         # Media, ListEntry, MediaEntry, PaginatedResponse
│   │   └── vault.rs         # VaultSubItemDto, VaultMetadataDto
│   └── workers/
│       └── downloader.rs    # Background download worker
└── tests/                   # Request, model, crawler, and adapter integration tests
```

## API Routes

All application routes are prefixed with `/api/v1`.

### User

| Method | Path                          | Description                                                    |
| ------ | ----------------------------- | -------------------------------------------------------------- |
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

| Method | Path                     | Description                                                     |
| ------ | ------------------------ | --------------------------------------------------------------- |
| `POST` | `/api/v1/crawler/search` | Search configured crawlers for media titles and parsed torrents |

### Vault (Downloads & Streaming)

| Method | Path                           | Description                                                            |
| ------ | ------------------------------ | ---------------------------------------------------------------------- |
| `POST` | `/api/v1/vault/add`            | Add a download item to the vault (direct URL or magnet/torrent)        |
| `GET`  | `/api/v1/vault/all`            | Server-Sent Events (SSE) stream for real-time vault download progress  |
| `POST` | `/api/v1/vault/pause`          | Pause an active direct or torrent download                             |
| `POST` | `/api/v1/vault/resume`         | Resume a paused download                                               |
| `POST` | `/api/v1/vault/delete`         | Delete / cancel a download and remove files from disk                  |
| `POST` | `/api/v1/vault/metadata`       | Batch fetch sub-items and relational metadata (tracks, subs, chapters) |
| `GET`  | `/api/v1/vault/stream/{*path}` | Stream media files, DASH manifests, fonts, and subtitles from vault    |

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

Configure provider credentials, storage locations, and transcoding paths in `.env` or via `config/*.yaml`:

| Variable                | Description                                       | Default                                 |
| ----------------------- | ------------------------------------------------- | --------------------------------------- |
| `PORT`                  | Server port                                       | `5150`                                  |
| `BINDING`               | Server bind address                               | `localhost`                             |
| `DATABASE_URL`          | SQLite database URI                               | `sqlite://assets/main.sqlite?mode=rwc`  |
| `QUEUE_URL`             | SQLite job queue URI                              | `sqlite://assets/queue.sqlite?mode=rwc` |
| `VAULT_LOC`             | Storage root directory for downloaded vault files | `vault`                                 |
| `ENCODED_LOC`           | Destination directory for transcoded media files  | `encoded`                               |
| `MAL_CLIENT_ID`         | MyAnimeList API client ID                         |                                         |
| `MAL_CLIENT_SECRET`     | MyAnimeList API client secret                     |                                         |
| `ANILIST_CLIENT_ID`     | AniList OAuth client ID                           |                                         |
| `ANILIST_CLIENT_SECRET` | AniList OAuth client secret                       |                                         |

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
