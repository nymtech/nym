#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::thread;

use rocket_contrib::serve::StaticFiles;

pub mod topology;
pub mod utils;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}
#[tokio::main]
async fn main() {
    thread::spawn(|| {
        rocket::ignite()
            .mount("/", StaticFiles::from("static"))
            .launch()
    });
    match topology::renew_periodically().await {
        Err(err) => println!("Error refreshing topology: {}", err),
        Ok(()) => println!("Refreshed topology"),
    };
}
