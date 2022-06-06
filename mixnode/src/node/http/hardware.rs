use rocket::serde::{json::Json, Serialize};
use sysinfo::{System, SystemExt};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct Hardware {
    ram: String,
    num_cores: usize,
    crypto_hardware: Option<CryptoHardware>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct CryptoHardware {
    aesni: bool,
    processor: String,
    sgx: bool,
}

/// Provides hardware information which Nym can use to optimize mixnet speed over time (memory, crypto hardware, CPU, cores, etc).
#[get("/hardware")]
pub(crate) fn hardware() -> Json<Hardware> {
    Json(hardware_info())
}

/// Gives back a summary report of whatever system hardware info we can get for this platform.
fn hardware_info() -> Hardware {
    let crypto_hardware = hardware_info_from_cupid();
    hardware_from_sysinfo(crypto_hardware)
}

/// Sysinfo gives back basic stuff like number of CPU cores and available memory. If available, this includes the hardware encryption
/// extensions report
fn hardware_from_sysinfo(crypto_hardware: Option<CryptoHardware>) -> Hardware {
    let mut system = System::new_all();
    let total_memory = system.free_memory();
    system.refresh_all();
    let ram = format!("{}KB", total_memory);
    let cores = system.cpus();
    let num_cores = cores.len();
    Hardware {
        crypto_hardware,
        ram,
        num_cores,
    }
}

/// The `cupid` crate gives back a report on available hardware encryption extensions which may be useful for future mixnet optimizations.
///
/// Note: this information is generally only available on x86 platforms for Linux.
fn hardware_info_from_cupid() -> Option<CryptoHardware> {
    cupid::master().map(|info| CryptoHardware {
        aesni: info.aesni(),
        processor: info.brand_string().unwrap().to_string(),
        sgx: info.sgx(),
    })
}
