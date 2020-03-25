use super::*;
use crate::services::mixmining;
use bodyparser::Struct;
use iron::status;

pub fn post(req: &mut Request) -> IronResult<Response> {
    // the part where we check if the worst possible handler code can work.
    let db = mixmining::db::MixminingDb::new();
    let service = mixmining::Service::new(db);

    let maybe_mixnode = match req.get::<Struct<mixmining::Mixnode>>() {
        Ok(Some(mixnode)) => Ok(mixnode),
        Ok(None) => Err("JSON parsing error"),
        Err(_) => Err("JSON parsing error"),
    };

    if maybe_mixnode.is_ok() {
        let mixnode = maybe_mixnode.unwrap();
        service.add(mixnode);
        Ok(Response::with(status::Created))
    } else {
        Ok(Response::with((
            status::BadRequest,
            maybe_mixnode.unwrap_err(),
        )))
    }
}
