use crate::discord::common::DatabaseContainer;
use crate::model::User;
use anyhow::anyhow;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

#[command]
pub async fn lurk(
    ctx: &serenity::prelude::Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    // let t = get_books().await?;

    let goodreads_id = args.single::<i64>()?;
    let data = ctx.data.read().await;
    if let Some(database) = data.get::<DatabaseContainer>() {
        let pool = &**database;
        let discord_user_id = msg.author.id.0 as i64;
        let discord_guild_id = msg
            .guild_id
            .ok_or(anyhow!("Expected a guild id on message"))?
            .0 as i64;
        match User::create_new_user(pool, discord_user_id, discord_guild_id, goodreads_id).await {
            Ok(_user) => {
                msg.reply(ctx, format!("You're in! type `~unlurk` to be removed."))
                    .await?;
            }
            Err(why) => {
                msg.reply(
                    ctx,
                    format!(
                        "Ooopsie! I was unable to add you to the _lurk list_ at this time :(\n{}",
                        why
                    ),
                )
                .await?;
                tracing::error!(
                    "Unable to add user ({}) in guild ({}) because: {}",
                    discord_user_id,
                    discord_guild_id,
                    why
                );
            }
        }
    }

    Ok(())
}
