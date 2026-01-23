-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Access tokens table
CREATE TABLE IF NOT EXISTS accesstokens (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    app_name TEXT NOT NULL,
    app_version TEXT NOT NULL,
    date_created TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_accesstokens_user_id ON accesstokens(user_id);

-- Items table
CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY,
    parent_id TEXT,
    collection_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sort_name TEXT,
    original_title TEXT,
    premiere_date TEXT,
    community_rating REAL,
    runtime_ticks INTEGER,
    production_year INTEGER,
    index_number INTEGER,
    parent_index_number INTEGER,
    item_type TEXT NOT NULL,
    date_created TEXT NOT NULL,
    date_modified TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_items_parent_id ON items(parent_id);
CREATE INDEX IF NOT EXISTS idx_items_collection_id ON items(collection_id);
CREATE INDEX IF NOT EXISTS idx_items_item_type ON items(item_type);

-- User playstate/data table
CREATE TABLE IF NOT EXISTS playstate (
    user_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    playback_position_ticks INTEGER NOT NULL DEFAULT 0,
    play_count INTEGER NOT NULL DEFAULT 0,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    last_played_date TEXT,
    PRIMARY KEY (user_id, item_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_playstate_user_id ON playstate(user_id);
CREATE INDEX IF NOT EXISTS idx_playstate_is_favorite ON playstate(user_id, is_favorite);
CREATE INDEX IF NOT EXISTS idx_playstate_last_played ON playstate(user_id, last_played_date);

-- Playlists table
CREATE TABLE IF NOT EXISTS playlist (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    date_created TEXT NOT NULL,
    date_modified TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_playlist_user_id ON playlist(user_id);

-- Playlist items table
CREATE TABLE IF NOT EXISTS playlist_item (
    playlist_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, item_id),
    FOREIGN KEY (playlist_id) REFERENCES playlist(id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_playlist_item_playlist_id ON playlist_item(playlist_id, sort_order);
