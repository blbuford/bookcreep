use chrono::offset::Utc;
use sqlx::sqlite::SqlitePool;

#[derive(Debug)]
pub struct User {
    pub discord_user_id: i64,
    pub goodreads_user_id: i64,
    pub last_etag: Option<String>,
    pub last_checked: i64,
    pub last_book_id: Option<String>,
}

impl User {
    pub fn new(
        discord_user_id: i64,
        goodreads_user_id: i64,
        last_etag: Option<String>,
        last_checked: i64,
        last_book_id: Option<String>,
    ) -> Self {
        Self {
            discord_user_id,
            goodreads_user_id,
            last_etag,
            last_checked,
            last_book_id,
        }
    }
    #[tracing::instrument(name = "Creating new user", skip(pool))]
    pub async fn add_user_to_db(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;
        sqlx::query!(
            r#"
            INSERT INTO users VALUES (?, ?, ?, ?, ?)
            "#,
            self.discord_user_id,
            self.goodreads_user_id,
            self.last_etag,
            self.last_checked,
            self.last_book_id,
        )
        .execute(&mut conn)
        .await?;

        Ok(())
    }
    #[tracing::instrument(
        name = "Getting all refreshable users",
        skip(pool, last_refreshed_minutes_ago)
    )]
    pub async fn get_refreshable_users(
        pool: &SqlitePool,
        last_refreshed_minutes_ago: i64,
    ) -> anyhow::Result<Option<Vec<User>>> {
        let mut conn = pool.acquire().await?;
        let timestamp = Utc::now().timestamp() - 60 * last_refreshed_minutes_ago;
        let results = sqlx::query!(r#"SELECT * FROM users WHERE last_checked < ?"#, timestamp)
            .fetch_all(&mut conn)
            .await?
            .iter()
            .map(|record| {
                User::new(
                    record.discord_user_id,
                    record.goodreads_user_id,
                    record.last_etag.to_owned(),
                    record.last_checked,
                    record.last_book_id.to_owned(),
                )
            })
            .collect::<Vec<User>>();

        if results.len() > 0 {
            Ok(Some(results))
        } else {
            Ok(None)
        }
    }
    #[tracing::instrument(name = "Updating user", skip(pool))]
    pub async fn update(&mut self, pool: &SqlitePool) -> anyhow::Result<()> {
        let mut conn = pool.acquire().await?;
        self.last_checked = Utc::now().timestamp();

        sqlx::query!(
            r#"UPDATE users SET last_book_id = ?, last_etag = ?, last_checked = ? WHERE discord_user_id = ?"#,
            self.last_book_id,
            self.last_etag,
            self.last_checked,
            self.discord_user_id
        )
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    // #[tracing::instrument(name = "Updating user (timestamp checked only)", skip(pool))]
    // pub async fn update_timestamp(&self, pool: &SqlitePool) -> anyhow::Result<()> {
    //     let mut conn = pool.acquire().await?;
    //     let now = Utc::now().timestamp();
    //     sqlx::query!(
    //         r#"UPDATE users SET last_checked = ?, lastWHERE discord_user_id = ?"#,
    //         now,
    //         self.discord_user_id
    //     )
    //     .execute(&mut conn)
    //     .await?;
    //
    //     Ok(())
    // }
    pub fn set_last_book_id(&mut self, id: Option<String>) {
        self.last_book_id = id;
    }

    pub fn set_last_etag(&mut self, etag: Option<String>) {
        self.last_etag = etag;
    }
}
