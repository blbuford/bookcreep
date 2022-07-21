use anyhow::Context;
use reqwest::Url;
use std::env;

use crate::model::{Book, User};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::prelude::AttachmentType;
use serenity::prelude::*;

// use crate::model::get_books;

#[group]
#[commands(ping)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[command]
async fn ping(ctx: &serenity::prelude::Context, msg: &Message) -> CommandResult {
    // let t = get_books().await?;

    msg.reply(ctx, "pannnic").await?;

    Ok(())
}

pub async fn get_discord_client() -> serenity::Client {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    client
}

#[tracing::instrument(name = "Posting message to discord", skip(http))]
pub async fn post_book(
    http: impl AsRef<Http>,
    book: &Book,
    user: &User,
) -> anyhow::Result<Message> {
    let rating = "⭐".repeat(book.rating());
    let msg = format!(
        "🎉\n <@{}> finished {} by {}",
        user.discord_user_id,
        book.title(),
        book.author()
    );
    let channel = ChannelId(996656225871740971);
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
