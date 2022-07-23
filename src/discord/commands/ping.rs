use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
pub async fn ping(ctx: &serenity::prelude::Context, msg: &Message) -> CommandResult {
    // let t = get_books().await?;

    msg.reply(ctx, "pannnic").await?;

    Ok(())
}
