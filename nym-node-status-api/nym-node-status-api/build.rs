use anyhow::Result;

/// If you need to re-run migrations or reset the db, just run
/// cargo clean -p nym-node-status-api
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        println!("cargo::rustc-env=DATABASE_URL={database_url}");
    }

    Ok(())
}
