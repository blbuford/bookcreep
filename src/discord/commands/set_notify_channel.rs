use crate::discord::common::DatabaseContainer;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use sqlx::SqlitePool;

#[command]
#[required_permissions(ADMINISTRATOR)]
pub async fn set_notify_channel(ctx: &serenity::prelude::Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    if let Some(database) = data.get::<DatabaseContainer>() {
        let pool = &**database;
        let notify_channel_id = msg.channel_id.0 as i64;
        let guild_id = msg.guild_id.expect("guild ID from msg").0 as i64;

        match update_notify_channel(pool, notify_channel_id, guild_id).await {
            Ok(_) => {
                msg.reply(
                    ctx,
                    format!(
                        "<#{}> has been set as the notification channel",
                        notify_channel_id
                    ),
                )
                .await?;
            }
            Err(why) => {
                tracing::error!(
                    "Unable to set the notify channel for guild ({}) because: {}",
                    guild_id,
                    why
                );
            }
        };
    }

    Ok(())
}
#[tracing::instrument(name = "Updating the notification channel in DB", skip(pool))]
async fn update_notify_channel(
    pool: &SqlitePool,
    notify_channel: i64,
    guild: i64,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    sqlx::query!(
        r#"UPDATE guilds SET notify_channel_id = ? WHERE guild_id = ?"#,
        notify_channel,
        guild
    )
    .execute(&mut conn)
    .await?;
    Ok(())
}
