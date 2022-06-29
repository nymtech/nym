use cupid::TopologyType;
use rocket::serde::{json::Json, Serialize};
use sysinfo::{System, SystemExt};

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub(crate) struct Hardware {
    ram: String,
    num_cores: usize,
    crypto_hardware: Option<CryptoHardware>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub(crate) struct CryptoHardware {
    aesni: bool,
    avx2: bool,
    brand_string: String,
    smt_logical_processor_count: Vec<u32>,
    osxsave: bool,
    sgx: bool,
    xsave: bool,
}

/// Provides hardware information which Nym can use to optimize mixnet speed over time (memory, crypto hardware, CPU, cores, etc).
#[get("/hardware")]
pub(crate) fn hardware() -> Json<Option<Hardware>> {
    Json(hardware_info())
}

/// Gives back a summary report of whatever system hardware info we can get for this platform.
fn hardware_info() -> Option<Hardware> {
    let crypto_hardware = hardware_info_from_cupid();
    hardware_from_sysinfo(crypto_hardware)
}

/// Sysinfo gives back basic stuff like number of CPU cores and available memory. If available, this includes the hardware encryption
/// extensions report
fn hardware_from_sysinfo(crypto_hardware: Option<CryptoHardware>) -> Option<Hardware> {
    if System::IS_SUPPORTED {
        let mut system = System::new_all();
        system.refresh_all();
        let ram = format!("{}KB", system.total_memory());
        let cores = system.cpus();
        let num_cores = cores.len();
        Some(Hardware {
            ram,
            num_cores,
            crypto_hardware,
        })
    } else {
        None
    }
}

/// The `cupid` crate gives back a report on available hardware encryption extensions which may be useful for future mixnet optimizations.
///
/// Note: this information is generally only available on x86 platforms for Linux.
fn hardware_info_from_cupid() -> Option<CryptoHardware> {
    cupid::master().map(|info| -> CryptoHardware {
        let smt_logical_processor_count =
            if let Some(extended_topology) = info.extended_topology_enumeration() {
                extended_topology
                    .clone()
                    .filter_map(|entry| {
                        if entry.level_type() == TopologyType::SMT {
                            Some(entry.logical_processor_count())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

        CryptoHardware {
            aesni: info.aesni(),
            avx2: info.avx2(),
            brand_string: info.brand_string().map(String::from).unwrap_or_default(),
            smt_logical_processor_count,
            osxsave: info.osxsave(),
            sgx: info.sgx(),
            xsave: info.xsave(),
        }
    })
}
