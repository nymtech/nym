use serde::{Deserialize, Serialize};

use super::*;
use bodyparser::Struct;
use iron::mime::Mime;
use iron::status;
use iron::Handler;

/// Holds data for a capacity update (json)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity {
    value: usize,
}

pub struct Update {
    service: Arc<Mutex<mixmining::Service>>,
}

impl Update {
    pub fn new(service: Arc<Mutex<mixmining::Service>>) -> Update {
        Update { service }
    }
}

impl Handler for Update {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let json_parse = req.get::<Struct<Capacity>>();

        if json_parse.is_ok() {
            let capacity = json_parse
                .unwrap()
                .expect("Unexpected JSON parsing problem")
                .value;
            self.service.lock().unwrap().set_capacity(capacity);
            Ok(Response::with(status::Created))
        } else {
            let error = json_parse.unwrap_err();
            Ok(Response::with((status::BadRequest, error.detail)))
        }
    }
}

pub struct Get {
    service: Arc<Mutex<mixmining::Service>>,
}

impl Get {
    pub fn new(service: Arc<Mutex<mixmining::Service>>) -> Get {
        Get { service }
    }
}

impl Handler for Get {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let content_type = "application/json".parse::<Mime>().unwrap();
        let value = self.service.lock().unwrap().capacity();
        let c = Capacity { value };
        let json = serde_json::to_string(&c).unwrap();
        Ok(Response::with((content_type, status::Ok, json)))
    }
}
