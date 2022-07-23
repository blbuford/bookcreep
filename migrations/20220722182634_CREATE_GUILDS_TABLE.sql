-- Add migration script here
CREATE TABLE guilds (
    guild_id            INT PRIMARY KEY NOT NULL,
    guild_name          TEXT            NOT NULL,
    notify_channel_id   INT             NOT NULL
);