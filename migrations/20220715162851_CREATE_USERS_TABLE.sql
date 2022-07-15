-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
    discord_user_id         INTEGER PRIMARY KEY NOT NULL,
    goodreads_user_id       INTEGER             NOT NULL,
    last_etag               TEXT                        ,
    last_checked            INTEGER             NOT NULL DEFAULT 0,
    last_isbn               TEXT
);