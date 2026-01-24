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

-- Items table (matching Go schema)
CREATE TABLE IF NOT EXISTS items (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    votes INTEGER,
    year INTEGER,
    genre TEXT NOT NULL,
    rating REAL,
    nfotime INTEGER NOT NULL,
    firstvideo INTEGER NOT NULL,
    lastvideo INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS items_name_idx ON items (name);

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
    FOREIGN KEY (playlistid) REFERENCES playlist(id)
);
