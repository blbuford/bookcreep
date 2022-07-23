use anyhow::{anyhow, Context};
use reqwest::Url;
use serenity::async_trait;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::prelude::{AttachmentType, Guild, GuildId};
use serenity::prelude::*;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;

use crate::discord::commands::*;
use crate::model::{Book, User};

pub struct DatabaseContainer;

impl TypeMapKey for DatabaseContainer {
    type Value = Arc<SqlitePool>;
}

#[group]
#[commands(ping, set_notify_channel)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: serenity::prelude::Context, ready: Ready) {
        tracing::info!("Connected as {}", ready.user.name);
        let data = ctx.data.read().await;
        if let Some(database) = data.get::<DatabaseContainer>() {
            let pool = &**database;
            for guild in ready.guilds.iter() {
                match verify_guild(pool, guild.id).await {
                    Ok(true) => {
                        // do nothing #shrug
                    }
                    Ok(false) => {
                        // add to db
                        if let Some(g) = guild.id.to_guild_cached(ctx.cache.clone()) {
                            if let Some(system_channel) = g.system_channel_id {
                                if let Err(why) = insert_guild(pool, &g, &system_channel).await {
                                    tracing::error!(
                                        "Unable to insert guild ({}) in database because: {}",
                                        g.id.0,
                                        why
                                    );
                                } else {
                                    // Post the initial help message
                                    let help = format!("👋\nTo chose which channel is used for notifications, (1) be an admin and (2) type `~set_notification_channel` in the channel that should have it.");
                                    if let Err(why) =
                                        system_channel.say(ctx.http.clone(), help).await
                                    {
                                        tracing::error!(
                                        "Unable to post initial help message in guild ({}) because: {}",
                                        g.id.0,
                                        why
                                    );
                                    }
                                }
                            }
                        }
                    }
                    Err(why) => {
                        tracing::error!(
                            "Unable to verify guild ({}) in database because: {}",
                            guild.id.0,
                            why
                        );
                    }
                }
            }
        }
    }

    async fn resume(&self, _: serenity::prelude::Context, _: ResumedEvent) {
        tracing::info!("Resumed");
    }
}

pub async fn get_discord_client(database: Arc<SqlitePool>) -> Client {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES;
    let client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    {
        let mut data = client.data.write().await;
        data.insert::<DatabaseContainer>(database)
    }
    client
}

#[tracing::instrument(name = "Posting message to discord", skip(http))]
pub async fn post_book(
    http: impl AsRef<Http>,
    book: &Book,
    user: &User,
    channel: ChannelId,
) -> anyhow::Result<Message> {
    let rating = "⭐".repeat(book.rating());
    let msg = format!(
        "🎉\n <@{}> finished {} by {}",
        user.discord_user_id,
        book.title(),
        book.author()
    );
    channel
        .send_message(&http, |m| {
            m.add_embed(|e| e.url(book.url()).description(rating).title("Review"))
                .content(msg)
                .add_file(AttachmentType::Image(
                    Url::parse(book.image()).expect("valid url struct for the book image"),
                ))
        })
        .await
        .with_context(|| format!("Unable to send message to discord channel {}", channel))
}

#[tracing::instrument(name = "Verifying guild is setup in database", skip(pool))]
async fn verify_guild(pool: &SqlitePool, guild: GuildId) -> anyhow::Result<bool> {
    let mut conn = pool.acquire().await?;
    let guild_id = guild.0 as i64;
    let res = sqlx::query!(r#"SELECT * FROM guilds WHERE guild_id = ?"#, guild_id)
        .fetch_one(&mut conn)
        .await;

    match res {
        Ok(_) => Ok(true),
        Err(sqlx::Error::RowNotFound) => Ok(false),
        Err(e) => Err(anyhow!(e)),
    }
}

#[tracing::instrument(
    name = "Inserting guild into database",
    skip(pool, guild, system_channel)
)]
async fn insert_guild(
    pool: &SqlitePool,
    guild: &Guild,
    system_channel: &ChannelId,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    let guild_id = guild.id.0 as i64;
    let guild_name = &guild.name;
    let system_channel_id = system_channel.0 as i64;
    sqlx::query!(
        r#"INSERT INTO guilds VALUES (?, ?, ?)"#,
        guild_id,
        guild_name,
        system_channel_id
    )
    .execute(&mut conn)
    .await?;

    Ok(())
}
