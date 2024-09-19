use crate::db::{models::JokeDto, DbPool};

pub(crate) async fn insert_joke(pool: &DbPool, joke: JokeDto) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;
    sqlx::query!(
        "INSERT INTO responses
                (joke_id, joke, date_created)
                VALUES
                ($1, $2, $3)
            ON CONFLICT(joke_id) DO UPDATE SET
            joke=excluded.joke,
            date_created=excluded.date_created;",
        joke.joke_id,
        joke.joke,
        joke.date_created as i32,
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub(crate) async fn select_joke_by_id(pool: &DbPool, joke_id: &str) -> anyhow::Result<JokeDto> {
    sqlx::query_as!(
        JokeDto,
        "SELECT joke_id, joke, date_created FROM responses WHERE joke_id = $1",
        joke_id
    )
    .fetch_one(pool)
    .await
    .map_err(anyhow::Error::from)
}

pub(crate) async fn select_all(pool: &DbPool) -> anyhow::Result<Vec<JokeDto>> {
    sqlx::query_as!(JokeDto, "SELECT joke_id, joke, date_created FROM responses",)
        .fetch_all(pool)
        .await
        .map_err(anyhow::Error::from)
}
