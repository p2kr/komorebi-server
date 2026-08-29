# komorebi-server

A high-performance, unified REST API backend written in Rust — built on the [Loco](https://loco.rs) framework. It aggregates and normalizes anime and manga list data across multiple third-party providers (MyAnimeList, AniList) into a single consistent API.

## Features

- **Multi-provider support** — MyAnimeList (REST) and AniList (GraphQL) behind a unified interface
- **Normalized data models** — scores, formats, statuses, and pagination unified regardless of provider
- **User management** — register, look up, and delete users linked to a provider account
- **Passcode auth** — optional Argon2-hashed passcode protection per user
- **OAuth support** — exchange OAuth authorization codes + PKCE verifiers for access tokens
- **Paginated list fetching** — anime and manga lists with filtering, sorting, and cursor/offset pagination
- **Background workers** — Loco worker queue for async download jobs
- **Crawler subsystem** — scrapes torrent sites (e.g. Nyaa.si) using configurable CSS-selector rules and title parsing
- **TypeScript bindings** — `ts-rs` emits TS types from Rust DTOs for frontend consumption

## Tech Stack

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

## Project Layout

```
komorebi-server/
├── config/                  # Per-environment YAML config (development / production / test)
├── docs/
│   ├── CONTEXT.md           # Architecture & codebase reference
│   └── openapi.yaml         # OpenAPI 3.1 specification
├── migration/               # Sea-ORM migrations
├── src/
│   ├── app.rs               # Loco Hooks impl — route & worker registration
│   ├── adapters/            # Provider client trait & implementations
│   │   ├── mod.rs           # MediaClient trait + MediaClientParams
│   │   ├── mal_client.rs    # MyAnimeList REST client
│   │   ├── mal_models.rs    # MAL DTO models & conversions
│   │   ├── anilist_client.rs  # AniList GraphQL client
│   │   └── anilist_models.rs  # AniList DTO models & conversions
│   ├── controllers/
│   │   ├── mod.rs               # Shared response envelopes (success / fail)
│   │   ├── media_controller.rs  # /api/v1/media routes
│   │   ├── user_controller.rs   # /api/v1/user routes
│   │   └── crawler_controller.rs # /api/v1/crawler routes (WIP)
│   ├── models/
│   │   ├── _entities/       # Generated Sea-ORM entities (do not edit)
│   │   ├── media.rs         # Core domain types (Media, ListEntry, enums)
│   │   ├── users.rs         # User model & DB operations
│   │   └── crawler.rs       # CrawlerConfig, CrawlerResult, ParsedTitle
│   ├── crawlers/            # Web scraping & title parsing subsystem
│   │   ├── html_crawler.rs  # CSS-selector HTML scraper
│   │   ├── json_crawler.rs  # JSON API scraper
│   │   ├── config_parser.rs # YAML crawler config loader
│   │   └── title_parser.rs  # Torrent title parser (Anitomy-style)
│   ├── dtos/                # TypeScript-exported DTO types (ts-rs)
│   ├── workers/
│   │   └── downloader.rs    # Background download worker
│   └── initializers/        # App-level initializers (reqwest client)
└── tests/                   # Integration & unit tests
```

## API Routes

All routes are prefixed with `/api/v1`.

### User

| Method | Path                   | Description                                                    |
| ------ | ---------------------- | -------------------------------------------------------------- |
| `POST` | `/user/login`          | Authenticate a user by username + provider + optional passcode |
| `POST` | `/user/add`            | Register / upsert a user (validates against provider)          |
| `POST` | `/user/all`            | List all registered users                                      |
| `POST` | `/user/one`            | Get a user by UUID                                             |
| `POST` | `/user/delete`         | Delete a user by UUID                                          |
| `POST` | `/user/oauth/exchange` | Exchange an OAuth code + PKCE verifier for an access token     |

### Media

| Method | Path           | Description                                          |
| ------ | -------------- | ---------------------------------------------------- |
| `POST` | `/media/anime` | Fetch a user's anime list from their linked provider |
| `POST` | `/media/manga` | Fetch a user's manga list from their linked provider |

### Crawler _(WIP)_

| Method | Path              | Description                                     |
| ------ | ----------------- | ----------------------------------------------- |
| `POST` | `/crawler/search` | Search external torrent sites for a media title |

### Response Envelopes

All responses use a consistent JSON envelope:

```json
// Success
{ "success": true, "data": { ... } }

// Failure
{ "success": false, "error": "ERROR_CODE", "description": "Optional detail" }
```

## Quick Start

```sh
# Apply database migrations
cargo loco db migrate

# Start the development server (http://localhost:5150)
cargo loco start
```

## Environment Variables

Configure provider credentials in your `.env` or via `config/development.yaml`:

| Variable                | Description                   |
| ----------------------- | ----------------------------- |
| `MAL_CLIENT_ID`         | MyAnimeList API client ID     |
| `MAL_CLIENT_SECRET`     | MyAnimeList API client secret |
| `ANILIST_CLIENT_ID`     | AniList OAuth client ID       |
| `ANILIST_CLIENT_SECRET` | AniList OAuth client secret   |

## Development

```sh
# Export TypeScript bindings for the web client
cargo ts-rs

# Run all tests
cargo test

# List all registered routes
cargo loco routes

# Check environment health
cargo loco doctor

# Apply database migrations
cargo loco db migrate
```

## Resources

- [Loco documentation](https://loco.rs/docs)
- [OpenAPI spec](docs/openapi.yaml)
- [Architecture context](docs/CONTEXT.md)
