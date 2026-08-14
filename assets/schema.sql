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
