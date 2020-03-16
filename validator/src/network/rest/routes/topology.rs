use super::*;

pub fn get(req: &mut Request) -> IronResult<Response> {
    let ref query = req
        .extensions
        .get::<Router>()
        .unwrap()
        .find("query")
        .unwrap_or("foomp");
    Ok(Response::with((status::Ok, *query)))
}
