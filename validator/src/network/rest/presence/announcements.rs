use super::*;
use crate::services::mixmining;
use iron::status;

use params::{Params, Value};

pub fn post(req: &mut Request) -> IronResult<Response> {
    // the part where we check if the worst possible handler code can work.
    let db = mixmining::db::MixminingDb::new();
    let service = mixmining::Service::new(db);
    let m = mixmining::Mixnode {
        public_key: "foo".to_string(),
        stake: 6,
    };
    service.add(m);

    // the actual params handling part.
    let map = req.get_ref::<Params>().unwrap();
    match map.find(&["user", "name"]) {
        Some(&Value::String(ref name)) if name == "Marie" => {
            Ok(Response::with((status::Ok, "Welcome back, Marie!")))
        }
        _ => Ok(Response::with(status::NotFound)),
    }
}
