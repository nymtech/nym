use iron::prelude::*;
use iron::status;

pub struct Api {}

impl Api {
    pub fn new() -> Api {
        Api {}
    }

    pub async fn run(self) {
        let port = 3000;
        println!("* starting REST API on localhost:{}", port);
        Iron::new(|_: &mut Request| Ok(Response::with((status::Ok, "Hello World!"))))
            .http(format!("localhost:{}", port))
            .unwrap();
    }
}
