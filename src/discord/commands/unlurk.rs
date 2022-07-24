use crate::discord::common::DatabaseContainer;
use crate::model::User;
use anyhow::anyhow;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::Message;

#[command]
pub async fn unlurk(ctx: &serenity::prelude::Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    if let Some(database) = data.get::<DatabaseContainer>() {
        let pool = &**database;
        let user_id = msg.author.id.0 as i64;
        let guild_id = msg
            .guild_id
            .ok_or(anyhow!("expected a guild attached to message"))?
            .0 as i64;

        match User::delete(pool, user_id, guild_id).await {
            Ok(_) => {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "<@{}>, you have been removed from the _lurk_ list!",
                            msg.author.id.0
                        ),
                    )
                    .await?;
            }
            Err(why) => {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("Sorry there's been an error :(\n{}", why),
                    )
                    .await?;
                tracing::error!("Error deleting user: {}", why);
            }
        }
    }

    Ok(())
}
