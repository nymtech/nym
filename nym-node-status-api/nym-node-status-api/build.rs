use anyhow::Result;

/// If you need to re-run migrations or reset the db, just run
/// cargo clean -p nym-node-status-api
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        String::from("postgres://testuser:testpass@localhost:5433/nym_node_status_api_test")
    });
    println!("cargo::rustc-env=DATABASE_URL={database_url}");

    // run migrations at build time to allow sqlx to fetch up-to-date DB schema
    let pool = sqlx::postgres::PgPool::connect(&database_url).await?;
    sqlx::migrate!("./migrations_pg").run(&pool).await?;
    pool.close().await;
    println!("cargo::rerun-if-changed=migrations");

    Ok(())
}
