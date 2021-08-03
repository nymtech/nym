use sqlx;
use std::env;
use std::fs;
use sqlx::SqliteConnection;
use sqlx::Connection;


#[tokio::main]
async fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    //   cargo:rustc-env=DATABASE_URL=sqlite:///home/dave/workspace/projects/nym/nym/validator-api/target/debug/build/nym-validator-api-0baae7e145508d83/out/validator-api-example.sqlite


    let database_path  = format!("sqlite://{}/validator-api-example.sqlite", out_dir);

    // println!("database_path is: {}", &database_path);

    println!("cargo:rustc-env=DATABASE_URL={}", &database_path);

    fs::File::create(format!("{}/validator-api-example.sqlite", &out_dir)).unwrap();

    // let opts = sqlx::sqlite::SqliteConnectOptions::new()
    //     .filename(&database_path)
    //     .create_if_missing(true);

    // let connection_pool = sqlx::SqlitePool::connect_with(opts).await.unwrap();

    let mut conn = SqliteConnection::connect(&database_path).await.unwrap();
    
    
    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .unwrap();
}
