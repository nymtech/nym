use super::*;
use crate::services::mixmining;
use bodyparser::Struct;
use iron::status;

pub fn post(req: &mut Request) -> IronResult<Response> {
    // the part where we check if the worst possible handler code can work.
    let db = mixmining::db::MixminingDb::new();
    let service = mixmining::Service::new(db);

    let json_parse = req.get::<Struct<mixmining::Mixnode>>();

    if json_parse.is_ok() {
        let mixnode = json_parse.unwrap().expect("No JSON supplied");
        service.add(mixnode);
        Ok(Response::with(status::Created))
    } else {
        let error = json_parse.unwrap_err();
        Ok(Response::with((status::BadRequest, error.detail)))
    }
}
