#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::thread;

use rocket_contrib::serve::StaticFiles;
mod jobs;
pub mod utils;

#[get("/")]
fn index() -> &'static str {
    "Later we will chop this up into multiple routes, but for now we'll just use StaticFiles. Leaving this here as a pointer for the future."
}

#[tokio::main]
async fn main() {
    thread::spawn(|| {
        rocket::ignite()
            .mount("/", StaticFiles::from("public"))
            .launch()
    });

    jobs::start().await;
}
