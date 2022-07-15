# Behavior
- Bot should alert that someone has finished reading a book on good reads.
- Bot should scan 'read' shelves of people who sign up to know when to alert (RSS)
- People should be able to use a slash command to add/remove themselves from the lurk list
- Bot should persist who its lurking, and the last seen 

# Schema

| discord-id | goodreads-id | last seen ETAG from goodreads | last checked datetime |
|----------------|----------------|----------------|----------------|
