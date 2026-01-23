-- Users table (matching Go schema)
CREATE TABLE IF NOT EXISTS users (
    id TEXT NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    created DATETIME,
    lastlogin DATETIME,
    lastused DATETIME
);

CREATE UNIQUE INDEX IF NOT EXISTS users_name_idx ON users (username);

-- Access tokens table (matching Go schema)
CREATE TABLE IF NOT EXISTS accesstokens (
    userid TEXT NOT NULL,
    token TEXT NOT NULL,
    deviceid TEXT,
    devicename TEXT,
    applicationname TEXT,
    applicationversion TEXT,
    remoteaddress TEXT,
    created DATETIME,
    lastused DATETIME
);

CREATE UNIQUE INDEX IF NOT EXISTS accesstokens_idx ON accesstokens (userid, token);

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

-- User playstate/data table (matching Go schema)
CREATE TABLE IF NOT EXISTS playstate (
    userid TEXT NOT NULL,
    itemid TEXT NOT NULL,
    position INTEGER,
    playedpercentage INTEGER,
    played BOOLEAN,
    playcount INTEGER,
    favorite BOOLEAN,
    timestamp DATETIME
);

CREATE UNIQUE INDEX IF NOT EXISTS userid_itemid_idx ON playstate (userid, itemid);

-- Playlists table (matching Go schema)
CREATE TABLE IF NOT EXISTS playlist (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    userid TEXT NOT NULL,
    timestamp DATETIME
);

-- Playlist items table (matching Go schema)
CREATE TABLE IF NOT EXISTS playlist_item (
    playlistid TEXT NOT NULL,
    itemid TEXT NOT NULL,
    itemorder INTEGER NOT NULL,
    timestamp DATETIME,
    PRIMARY KEY (playlistid, itemid),
    FOREIGN KEY (playlistid) REFERENCES playlists(id)
);
