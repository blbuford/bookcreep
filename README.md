# Book Creep
Book Creep is a discord bot that posts notifications of subscribed users' completed books (as recorded on each user's good reads page). 

# Usage
Clone this repository, and run 

```DISCORD_TOKEN=<your-secret-token> DATABASE_URL=sqlite:bookcreep.db RUST_LOG="sqlx=error,serenity=error,info" cargo run```

Docker build/run works as well, so long as you pass in the 3 environment variables above.
# Commands
`~set_notify_channel` - Administrators can run this command in the channel they wish the bot to post in

`~lurk <goodreads-id>` - @everyone can run this to subscribe themselves to the bot and have their completed books posted

`~unlurk` - @everyone can run this to unsubscribe themselves.

`~help` - a simple help command that contains this information