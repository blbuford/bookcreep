-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    discord_user_id         INTEGER             NOT NULL,
    discord_guild_id        INTEGER             NOT NULL,
    goodreads_user_id       INTEGER             NOT NULL,
    last_etag               TEXT                        ,
    last_checked            INTEGER             NOT NULL DEFAULT 0,
    last_book_id            TEXT
);