use super::*;
use crate::services::mixmining;
use bodyparser::Struct;
use iron::status;
use iron::Handler;

pub struct MixnodeHandler {
    service: mixmining::Service,
}

impl MixnodeHandler {
    pub fn new(service: mixmining::Service) -> MixnodeHandler {
        MixnodeHandler { service }
    }
}

impl Handler for MixnodeHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let json_parse = req.get::<Struct<mixmining::Mixnode>>();

        if json_parse.is_ok() {
            let mixnode = json_parse
                .unwrap()
                .expect("Unexpected JSON parsing problem");
            self.service.add(mixnode);
            Ok(Response::with(status::Created))
        } else {
            let error = json_parse.unwrap_err();
            Ok(Response::with((status::BadRequest, error.detail)))
        }
    }
}
