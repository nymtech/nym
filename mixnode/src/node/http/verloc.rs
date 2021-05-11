use rocket_contrib::json::Json;
use serde::Serialize;

/// Provides verifiable location (verloc) measurements for this mixnode - a list of the
/// round-trip times, in milliseconds, for all other mixnodes that this node knows about.
#[get("/verloc")]
pub(crate) fn verloc() -> Json<Vec<Foomp>> {
    // replace the foomps with a reference to the measurements vec :)
    let foomp1 = Foomp {
        ip: "1.2.3.4".to_string(),
        port: "1789".to_string(),
        identity_key: "abc".to_string(),
    };
    let foomp2 = Foomp {
        ip: "2.3.4.5".to_string(),
        port: "1789".to_string(),
        identity_key: "def".to_string(),
    };
    let foomps = vec![foomp1, foomp2];
    Json(foomps)
}

#[derive(Serialize)]
pub(crate) struct Foomp {
    ip: String,
    port: String,
    identity_key: String,
}
