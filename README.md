# komorebi-server

Backend API service and data synchronization engine for Komorebi.

`komorebi-server` handles communication with external media providers (AniList and MyAnimeList), normalizes disparate data models into a unified schema, manages user accounts in a local SQLite database, and optionally serves the web client as a single deployable binary.

---

## Core Capabilities

- **Provider Integration & Aggregation**:
  - Connects to **AniList** (GraphQL API) and **MyAnimeList** (REST API v2).
  - Fetches and paginates anime and manga lists per user.
- **Data Normalization Engine**:
  - Unifies score ratings across services to a standard `0.0–10.0` floating-point scale.
  - Normalizes media types (`Anime`, `Manga`), release formats (`Tv`, `Movie`, `Special`, `Ova`, `Ona`, `Manga`, `Novel`, `OneShot`, etc.), and airing/publishing statuses.
  - Standardizes watch/read statuses (`Current`, `Planning`, `Completed`, `Dropped`, `Paused`, `Repeating`).
- **Account Management**:
  - Manages user profiles with SQLite persistence.
  - Handles OAuth authorization code exchange for authenticated accounts.
  - Supports read-only Sandbox mode for public user list lookups without API tokens.
- **Embedded Web Serving**:
  - Embeds static assets compiled from `komorebi-web` and serves them with client-side SPA routing fallback.
- **Predictable API Responses**:
  - All endpoints return consistent `{ "success": true, "data": ... }` or standardized error envelopes.

---

## Directory Overview

```
komorebi-server/
├── assets/
│   └── schema.sql              # Database schema definitions & triggers
├── docs/
│   ├── CONTEXT.md              # Deep technical context & design documentation
│   └── openapi.yaml            # OpenAPI 3.1 schema specification
├── src/
│   ├── main.rs                 # Server entrypoint and graceful shutdown
│   ├── lib.rs                  # Library crate root
│   ├── adapters/               # AniList and MyAnimeList client implementations
│   ├── core/                   # Server config, state, startup, telemetry
│   ├── db/                     # SQLite pool and User repository CRUD
│   ├── handlers/               # Route handlers (media, user, oauth, SPA)
│   ├── models/                 # Domain entities and standardized models
│   └── services/               # List fetching and provider orchestration
└── tests/                      # Integration and model unit tests
```

---

## Configuration

Set configuration variables in a `.env` file in the `komorebi-server` directory:

```env
# Database location (defaults to local sqlite file)
DATABASE_URL=sqlite://assets/dev.db

# Provider API Credentials (used for OAuth token exchange)
MAL_CLIENT_ID=your_mal_client_id
MAL_CLIENT_SECRET=your_mal_client_secret

ANILIST_CLIENT_ID=your_anilist_client_id
ANILIST_CLIENT_SECRET=your_anilist_client_secret
```

---

## API Endpoints

All endpoints are hosted under `/api/v1`:

| Method | Route | Description |
| :--- | :--- | :--- |
| `POST`/`GET` | `/api/v1/health` | Service health status, version, and uptime |
| `POST` | `/api/v1/media/anime` | Fetch user anime entries with pagination & status filters |
| `POST` | `/api/v1/media/manga` | Fetch user manga entries with pagination & status filters |
| `POST` | `/api/v1/user/add` | Save or update a user account (Sandbox or OAuth) |
| `POST` | `/api/v1/user/all` | List all saved user accounts |
| `POST` | `/api/v1/user/one` | Retrieve user account by ID |
| `POST` | `/api/v1/user/delete` | Remove a user account by ID |
| `POST` | `/api/v1/oauth/exchange` | Exchange an OAuth authorization code for an access token |

---

## Development & Testing

### Running Tests

```bash
cargo test
```

### Running the Server

```bash
cargo run
```

The server listens at `127.0.0.1:8080`.

### Building for Production

```bash
cargo build --release
```
