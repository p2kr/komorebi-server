-- Database Schema for komorebi-server (SQLite)
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

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_updated_at ON users(updated_at);

CREATE TRIGGER IF NOT EXISTS users_au
AFTER UPDATE ON users
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_vault_source_url ON vault(source_url);
CREATE INDEX IF NOT EXISTS idx_vault_media_type ON vault(media_type);
CREATE INDEX IF NOT EXISTS idx_vault_status ON vault(status);
CREATE INDEX IF NOT EXISTS idx_vault_user_id ON vault(user_id);
CREATE INDEX IF NOT EXISTS idx_vault_updated_at ON vault(updated_at);

CREATE TRIGGER IF NOT EXISTS vault_au
AFTER UPDATE ON vault
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE vault SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_vault_sub_item_vault_id ON vault_sub_item(vault_id);
CREATE INDEX IF NOT EXISTS idx_vault_sub_item_media_type ON vault_sub_item(media_type);
CREATE INDEX IF NOT EXISTS idx_vault_sub_item_status ON vault_sub_item(status);
CREATE INDEX IF NOT EXISTS idx_vault_sub_item_updated_at ON vault_sub_item(updated_at);

CREATE TRIGGER IF NOT EXISTS vault_sub_item_au
AFTER UPDATE ON vault_sub_item
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE vault_sub_item SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_vault_metadata_sub_item_id ON vault_metadata(sub_item_id);
CREATE INDEX IF NOT EXISTS idx_vault_metadata_file_name ON vault_metadata(file_name);

CREATE TRIGGER IF NOT EXISTS vault_metadata_au
AFTER UPDATE ON vault_metadata
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE vault_metadata SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_audio_tracks_metadata_id ON audio_tracks(metadata_id);
CREATE INDEX IF NOT EXISTS idx_audio_tracks_language ON audio_tracks(language);
CREATE INDEX IF NOT EXISTS idx_audio_tracks_title ON audio_tracks(title);

CREATE TRIGGER IF NOT EXISTS audio_tracks_au
AFTER UPDATE ON audio_tracks
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE audio_tracks SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_video_subtitles_metadata_id ON video_subtitles(metadata_id);
CREATE INDEX IF NOT EXISTS idx_video_subtitles_language ON video_subtitles(language);
CREATE INDEX IF NOT EXISTS idx_video_subtitles_title ON video_subtitles(title);

CREATE TRIGGER IF NOT EXISTS video_subtitles_au
AFTER UPDATE ON video_subtitles
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE video_subtitles SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_video_chapters_metadata_id ON video_chapters(metadata_id);

CREATE TRIGGER IF NOT EXISTS video_chapters_au
AFTER UPDATE ON video_chapters
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE video_chapters SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

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

CREATE INDEX IF NOT EXISTS idx_subtitle_fonts_metadata_id ON subtitle_fonts(metadata_id);

CREATE TRIGGER IF NOT EXISTS subtitle_fonts_au
AFTER UPDATE ON subtitle_fonts
FOR EACH ROW
WHEN OLD.updated_at <> CURRENT_TIMESTAMP -- Prevents infinite looping
BEGIN
    UPDATE subtitle_fonts SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;
