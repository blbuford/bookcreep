use chrono::offset::Utc;
use sqlx::sqlite::SqlitePool;

#[derive(Debug)]
pub struct User {
    pub discord_user_id: i64,
    pub goodreads_user_id: i64,
    pub last_etag: Option<String>,
    pub last_checked: i64,
    pub last_isbn: Option<String>,
}

impl User {
    pub fn new(
        discord_user_id: i64,
        goodreads_user_id: i64,
        last_etag: Option<String>,
        last_checked: i64,
        last_isbn: Option<String>,
    ) -> Self {
        Self {
            discord_user_id,
            goodreads_user_id,
            last_etag,
            last_checked,
            last_isbn,
        }
    }
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
            self.last_isbn,
        )
        .execute(&mut conn)
        .await?;

        Ok(())
    }
    pub async fn get_refreshable_users(pool: &SqlitePool, last_refreshed_minutes_ago: i64) -> anyhow::Result<Option<Vec<User>>> {
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
                    record.last_isbn.to_owned(),
                )
            })
            .collect::<Vec<User>>();

        if results.len() > 0 {
            Ok(Some(results))
        } else {
            Ok(None)
        }
    }
}
