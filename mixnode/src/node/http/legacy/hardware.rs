// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::Query;
use cupid::TopologyType;
use nym_http_api_common::{FormattedResponse, OutputParams};
use serde::Serialize;
use sysinfo::System;

#[derive(Serialize, Debug)]
pub struct Hardware {
    ram: String,
    num_cores: usize,
    crypto_hardware: Option<CryptoHardware>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Debug)]
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
pub(crate) async fn hardware(Query(output): Query<OutputParams>) -> MixnodeHardwareResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(hardware_info())
}

pub type MixnodeHardwareResponse = FormattedResponse<Option<Hardware>>;

/// Gives back a summary report of whatever system hardware info we can get for this platform.
fn hardware_info() -> Option<Hardware> {
    let crypto_hardware = hardware_info_from_cupid();
    hardware_from_sysinfo(crypto_hardware)
}

/// Sysinfo gives back basic stuff like number of CPU cores and available memory. If available, this includes the hardware encryption
/// extensions report
fn hardware_from_sysinfo(crypto_hardware: Option<CryptoHardware>) -> Option<Hardware> {
    if sysinfo::IS_SUPPORTED_SYSTEM {
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
