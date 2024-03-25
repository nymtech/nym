// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cupid::TopologyType;
use nym_node_http_api::api::api_requests::v1::node::models::{
    Cpu, CryptoHardware, Hardware, HostSystem,
};
use sysinfo::System;

fn crypto_hardware() -> Option<CryptoHardware> {
    let Some(info) = cupid::master() else {
        return None;
    };
    let Some(extended_topology) = info.extended_topology_enumeration() else {
        return None;
    };

    let smt_logical_processor_count = extended_topology
        .clone()
        .filter_map(|leaf| {
            if leaf.level_type() == TopologyType::SMT {
                Some(leaf.logical_processor_count())
            } else {
                None
            }
        })
        .collect();

    Some(CryptoHardware {
        aesni: info.aesni(),
        avx2: info.avx2(),
        smt_logical_processor_count,
        osxsave: info.osxsave(),
        sgx: info.sgx(),
        xsave: info.xsave(),
    })
}

pub(crate) fn get_system_info(
    allow_hardware_info: bool,
    allow_crypto_hardware_info: bool,
) -> HostSystem {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        return Default::default();
    }

    let mut system = sysinfo::System::new_all();
    // TODO: do we actually need to refresh all here?
    system.refresh_all();

    let crypto_hardware = if allow_crypto_hardware_info && allow_hardware_info {
        crypto_hardware()
    } else {
        None
    };

    let hardware = if allow_hardware_info {
        let cpu = system
            .cpus()
            .iter()
            .map(|cpu| Cpu {
                name: cpu.name().to_string(),
                frequency: cpu.frequency(),
                vendor_id: cpu.vendor_id().to_string(),
                brand: cpu.brand().to_string(),
            })
            .collect();
        Some(Hardware {
            cpu,
            total_memory: system.total_memory(),
            crypto: crypto_hardware,
        })
    } else {
        None
    };

    HostSystem {
        system_name: System::name(),
        kernel_version: System::kernel_version(),
        os_version: System::os_version(),
        cpu_arch: System::cpu_arch(),
        hardware,
    }
}
