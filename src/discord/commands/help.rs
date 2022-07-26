use crate::discord::common::HELP_STR;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;

#[command]
pub async fn help(ctx: &serenity::prelude::Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, HELP_STR).await?;

    Ok(())
}
