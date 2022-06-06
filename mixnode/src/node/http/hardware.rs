use rocket::serde::{json::Json, Serialize};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct Hardware {
    aesni: bool,
    processor: String,
    sgx: bool,
}

/// Provides hardware information which Nym can use to optimize mixnet speed over time (memory, crypto hardware, CPU, cores, etc).
#[get("/hardware")]
pub(crate) fn hardware() -> Json<Option<Hardware>> {
    if let Some(info) = hardware_info() {
        Json(Some(info))
    } else {
        Json(None)
    }
}

fn hardware_info() -> Option<Hardware> {
    cupid::master().map(|info| Hardware {
        aesni: info.aesni(),
        processor: info.brand_string().unwrap().to_string(),
        sgx: info.sgx(),
    })
}
