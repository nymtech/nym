use iron::prelude::*;
use iron::status;

pub struct Api {}

impl Api {
    pub fn new() -> Api {
        Api {}
    }

    pub async fn run(self) {
        Iron::new(|_: &mut Request| Ok(Response::with((status::Ok, "Hello World!"))))
            .http("localhost:3000")
            .unwrap();
    }
}
