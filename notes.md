# Behavior
- Bot should alert that someone has finished reading a book on good reads.
- Bot should scan 'read' shelves of people who sign up to know when to alert (RSS)
- People should be able to use a slash command to add/remove themselves from the lurk list
- Bot should persist who its lurking, and the last seen 

# Schema

| discord-id | goodreads-id | last seen ETAG from goodreads | last checked datetime |
|----------------|----------------|----------------|----------------|

# Todo
- Unit tests for the RSS crawler
- Discord commands for a user to add/remove themselves from the crawl
- Bot configuration--what channel does it post in. Admin allowed to choose a channel to work in. 
- Containerization
- Deployment