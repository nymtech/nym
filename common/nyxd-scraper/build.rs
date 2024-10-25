#[tokio::main]
async fn main() {
    use sqlx::{Connection, Executor, PgConnection};

    const POSTGRES_USER: &str = "nym";
    const POSTGRES_PASSWORD: &str = "password123";
    const POSTGRES_DB: &str = "nyxd_scraper";

    let admin_url = format!(
        "postgres://{}:{}@localhost:5432/postgres",
        POSTGRES_USER, POSTGRES_PASSWORD
    );
    // Connect to postgres to create test database
    let database_url =
        format!("postgres://{POSTGRES_USER}:{POSTGRES_PASSWORD}@localhost:5432/{POSTGRES_DB}");

    let mut conn = PgConnection::connect(&admin_url)
        .await
        .expect("Failed to connect to Postgres");

    conn.execute(format!(r#"DROP DATABASE IF EXISTS {}"#, POSTGRES_DB).as_str())
        .await
        .expect("Failed to drop test database");

    conn.execute(format!(r#"CREATE DATABASE {}"#, POSTGRES_DB).as_str())
        .await
        .expect("Failed to create test database");

    let mut test_conn = PgConnection::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./sql_migrations")
        .run(&mut test_conn)
        .await
        .expect("Failed to perform SQLx migrations");

    // Set the database URL as an environment variable
    println!("cargo:rustc-env=DATABASE_URL={}", database_url);
}
