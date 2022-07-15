use sqlx::sqlite::SqlitePool;

#[derive(Debug)]
pub struct User {
    discord_user_id: i64,
    goodreads_user_id: i64,
    last_etag: Option<String>,
    last_checked: i64,
    last_isbn: Option<String>,
}

impl User {
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
}
