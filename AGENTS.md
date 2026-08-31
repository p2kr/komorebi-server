# Agent guide for this Loco app

This is a **Loco** (loco.rs) application — an all-in-one, batteries-included Rust
web framework. Routing, the database (Sea-ORM), background jobs, a scheduler,
mailers, tasks, storage, caching, and testing are already integrated. **Prefer
Loco's built-ins and generators over adding external crates or wiring
infrastructure by hand.**

## Where things live

```
src/app.rs            # impl Hooks for App — registers routes/workers/tasks (the wiring hub)
src/controllers/      # HTTP handlers grouped into Routes (media, user, crawler, vault)
src/models/_entities/ # GENERATED Sea-ORM entities — do not hand-edit
src/models/*.rs       # your model logic (media.rs, users.rs, crawler.rs, vault.rs)
src/adapters/         # MediaClient trait + MAL & AniList provider implementations
src/crawlers/         # Crawler & TitleParser traits + HTML/JSON scraper implementations
src/downloaders/      # DownloadManager, DownloadEngine trait (Direct, Torrent), polling daemon
src/dtos/             # ts-rs DTO types exported to frontend/src/bindings/
src/core/             # App-wide constants, path resolvers, client builder, and utilities
src/workers/          # background jobs (DownloadWorker)
src/tasks/            # CLI/admin tasks
migration/            # Sea-ORM migrations
config/*.yaml         # per-environment config (LOCO_ENV)
tests/                # request/model/task tests
docs/CONTEXT.md       # architecture & codebase reference (read this first)
docs/openapi.yaml     # OpenAPI 3.1 spec
```

## How to work in this app

- **Add features with generators**, then edit:
  `cargo loco generate model|scaffold|controller|worker|task|mailer|migration ...`.
  The generators also wire new code into `src/app.rs`.
- **Everything uses `AppContext` (`ctx`)**: `ctx.db`, `ctx.config`,
  `ctx.cache`, `ctx.queue_provider`. Don't create your own DB pool, server, or job queue.
- Start every controller/model/worker/task with `use loco_rs::prelude::*;`.
- App code returns `loco_rs::Result<T>` and uses `?`.
- Config is YAML in `config/`; secrets come from the environment via the
  `get_env` Tera helper inside the YAML.
- Primary keys on `users` and `vault` are **UUID v7** (`uuid::Uuid`), not sequential integers.

## Project-specific conventions

- **Shared store state**: `ctx.shared_store` holds:
  - `Client`: Shared `reqwest::Client` from `src/core/client.rs`. Retrieve with `ctx.shared_store.get::<Client>().unwrap()`.
  - `Arc<DownloadManager>`: Central download orchestrator. Retrieve with `ctx.shared_store.get::<Arc<DownloadManager>>().unwrap()`.
  - `Sender<Vec<VaultItem>>`: Tokio broadcast channel for real-time WebSocket progress updates. Retrieve with `ctx.shared_store.get::<Sender<Vec<VaultItem>>>().unwrap()`.
- **Provider dispatch**: Use `user.provider.new_client(&client, &user)` to get a
  `Box<dyn MediaClient>`. Never instantiate `MalClient` or `AniListClient` directly.
- **Response envelopes**: All handlers return via the `success(data)` or
  `fail(status, "ERROR_CODE", description)` helpers in `src/controllers/mod.rs`.
  Do not return raw JSON outside of these wrappers.
- **User model**: `User` is an alias for `users::Model`. Always go through
  `User::save_user` for upserts — it runs `ActiveModelBehavior::before_save`
  which sets UUID, `is_sandbox`, and timestamps.
- **Vault model**: `VaultItem` is an alias for `vault::Model`. Use `ActiveModelBehavior::before_save`
  which sets UUID v7 and timestamps. Isolated storage lives at `{VAULT_LOC}/{vault_id}/`.
- **Crawler configs**: Load via `config_parser` or fall back to `CrawlerConfig::fallback()`
  (Nyaa.si). Do not hardcode selectors in controller logic.
- **TypeScript bindings**: Structs in `src/dtos/` that derive `ts-rs::TS` and annotate
  `#[ts(export)]` are auto-exported to `frontend/src/bindings/` at build time.
  Run `cargo build` to regenerate after changing DTOs.

## Useful commands

```
cargo loco start            # run the app (http://localhost:5150)
cargo loco db migrate       # apply migrations
cargo loco routes           # list all registered routes
cargo loco task <name>      # run a CLI task
cargo loco doctor           # check the environment
cargo test                  # run all tests
```

## Learn more

- Architecture reference: `docs/CONTEXT.md`
- Framework agent guide: https://loco.rs/AGENTS.md
- Full single-file reference: https://loco.rs/llms-full.txt
- Docs: https://loco.rs/docs
