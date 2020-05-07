use super::*;
use crate::network::rest::presence::models::Mixnode as PresenceMixnode;
use crate::services::mixmining::models::Mixnode as ServiceMixnode;
use bodyparser::Struct;
use iron::status;
use iron::Handler;
use models::Timestamp;

pub struct CreatePresence {
    service: Arc<Mutex<mixmining::Service>>,
}

impl CreatePresence {
    pub fn new(service: Arc<Mutex<mixmining::Service>>) -> CreatePresence {
        CreatePresence { service }
    }
}

impl Handler for CreatePresence {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let json_parse = req.get::<Struct<PresenceMixnode>>();

        if json_parse.is_ok() {
            let mixnode = json_parse
                .unwrap()
                .expect("Unexpected JSON parsing problem");
            self.service
                .lock()
                .unwrap()
                .add(ServiceMixnode::from_rest_mixnode_with_timestamp(
                    mixnode,
                    Timestamp::default(),
                ));
            Ok(Response::with(status::Created))
        } else {
            let error = json_parse.unwrap_err();
            Ok(Response::with((status::BadRequest, error.detail)))
        }
    }
}
