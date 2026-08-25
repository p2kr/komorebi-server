-- Database Schema for komorebi-server

CREATE TABLE IF NOT EXISTS users (
    id BLOB PRIMARY KEY NOT NULL,
    provider_id TEXT,
    username TEXT NOT NULL,
    avatar_url TEXT,
    provider TEXT NOT NULL,
    is_sandbox BOOLEAN NOT NULL DEFAULT 1,
    access_token TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    UNIQUE(username, provider, is_sandbox)
);

CREATE TRIGGER IF NOT EXISTS au_users
AFTER UPDATE ON users
FOR EACH ROW
BEGIN
    UPDATE users SET updated_at = (unixepoch('subsec') * 1000) WHERE id = OLD.id;
END;

-- ------------------------------
CREATE TABLE IF NOT EXISTS vault (
    id                BLOB    PRIMARY KEY NOT NULL, -- UUID v7
    user_id           BLOB    NOT NULL,             -- Foreign key to users.id
    media_id          TEXT,                         -- Optional provider media ID (e.g. MAL / AniList ID)
    title             TEXT    NOT NULL,             -- Display title or file name
    source_url        TEXT    NOT NULL,             -- Download URL, magnet link, or torrent URL
    download_type     TEXT    NOT NULL,             -- 'HTTP', 'MAGNET', 'TORRENT'
    status            TEXT    NOT NULL,             -- 'PENDING', 'DOWNLOADING', 'PAUSED', 'COMPLETED', 'FAILED', 'CANCELLED'
    total_bytes       INTEGER NOT NULL DEFAULT 0,   -- Total file size in bytes
    downloaded_bytes  INTEGER NOT NULL DEFAULT 0,   -- Bytes downloaded so far
    progress          REAL    NOT NULL DEFAULT 0.0, -- 0.0 to 100.0
    speed_bps         INTEGER NOT NULL DEFAULT 0,   -- Transfer speed in bytes per second
    eta_seconds       INTEGER,                      -- Estimated seconds remaining
    destination_path  TEXT    NOT NULL,             -- Final path in user's Vault
    temp_path         TEXT,                         -- Temporary partial file path (.part)
    error_msg         TEXT,                         -- Error description on failure
    created_at        INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    updated_at        INTEGER NOT NULL DEFAULT (unixepoch('subsec') * 1000),
    completed_at      INTEGER,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_vault_user_id ON vault(user_id);
CREATE INDEX idx_vault_status ON vault(status);

CREATE TRIGGER IF NOT EXISTS au_vault
AFTER UPDATE ON vault
FOR EACH ROW
BEGIN
    UPDATE vault SET updated_at = (unixepoch('subsec') * 1000) WHERE id = OLD.id;
END;
