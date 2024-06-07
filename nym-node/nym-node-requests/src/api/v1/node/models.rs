// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use celes::Country;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub use crate::api::SignedHostInformation;
pub use nym_bin_common::build_information::BinaryBuildInformationOwned;

#[derive(Clone, Default, Debug, Copy, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeRoles {
    pub mixnode_enabled: bool,
    pub gateway_enabled: bool,
    pub network_requester_enabled: bool,
    pub ip_packet_router_enabled: bool,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HostInformation {
    /// Ip address(es) of this host, such as `1.1.1.1`.
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>, format = Byte, example = json!(["1.1.1.1"])))]
    pub ip_address: Vec<IpAddr>,

    /// Optional hostname of this node, for example `nymtech.net`.
    #[cfg_attr(feature = "openapi", schema(example = "nymtech.net"))]
    pub hostname: Option<String>,

    /// Public keys associated with this node.
    pub keys: HostKeys,
}

#[derive(Serialize)]
pub struct LegacyHostInformation {
    pub ip_address: Vec<IpAddr>,
    pub hostname: Option<String>,
    pub keys: LegacyHostKeys,
}

impl From<HostInformation> for LegacyHostInformation {
    fn from(value: HostInformation) -> Self {
        LegacyHostInformation {
            ip_address: value.ip_address,
            hostname: value.hostname,
            keys: value.keys.into(),
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HostKeys {
    /// Base58-encoded ed25519 public key of this node. Currently, it corresponds to either mixnode's or gateway's identity.
    #[serde(alias = "ed25519")]
    pub ed25519_identity: String,

    /// Base58-encoded x25519 public key of this node used for sphinx/outfox packet creation.
    /// Currently, it corresponds to either mixnode's or gateway's key.
    #[serde(alias = "x25519")]
    pub x25519_sphinx: String,

    /// Base58-encoded x25519 public key of this node used for the noise protocol.
    #[serde(default)]
    pub x25519_noise: String,
}

impl From<HostKeys> for LegacyHostKeys {
    fn from(value: HostKeys) -> Self {
        LegacyHostKeys {
            ed25519: value.ed25519_identity,
            x25519: value.x25519_sphinx,
        }
    }
}

#[derive(Serialize)]
pub struct LegacyHostKeys {
    pub ed25519: String,
    pub x25519: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HostSystem {
    /// Name of the operating system of the host machine.
    pub system_name: Option<String>,

    /// Version of the kernel of the host machine, if applicable.
    pub kernel_version: Option<String>,

    /// Version of the operating system of the host machine, if applicable.
    pub os_version: Option<String>,

    /// The CPU architecture of the host machine (eg. x86, amd64, aarch64, ...).
    pub cpu_arch: Option<String>,

    /// Hardware information of the host machine.
    pub hardware: Option<Hardware>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Hardware {
    /// The information of the host CPU.
    pub cpu: Vec<Cpu>,

    /// Total memory, in bytes, available on the host.
    pub total_memory: u64,

    /// Detailed information about availability of crypto-specific instructions for future optimisations.
    pub crypto: Option<CryptoHardware>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Cpu {
    pub name: String,

    /// The CPU frequency in MHz.
    pub frequency: u64,

    pub vendor_id: String,

    pub brand: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CryptoHardware {
    /// Flag to indicate whether the host machine supports AES-NI x86 extension instruction set
    pub aesni: bool,

    /// Flag to indicate whether the host machine supports AVX2 x86 extension instruction set
    pub avx2: bool,

    /// Number of SMT logical processors available.
    pub smt_logical_processor_count: Vec<u32>,

    /// Flag to indicate whether the host machine supports OSXSAVE instruction
    pub osxsave: bool,

    /// Flag to indicate whether the host machine supports Intel Software Guard Extensions (SGX) set of instruction codes
    pub sgx: bool,

    /// Flag to indicate whether the host machine supports XSAVE instruction
    pub xsave: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeDescription {
    /// moniker defines a human-readable name for the node.
    pub moniker: String,

    /// website defines an optional website link.
    pub website: String,

    /// security contact defines an optional email for security contact.
    pub security_contact: String,

    /// details define other optional details.
    pub details: String,
}

/// Auxiliary details of the associated Nym Node.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AuxiliaryDetails {
    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[cfg_attr(feature = "openapi", schema(example = "PL", value_type = Option<String>))]
    #[schemars(with = "Option<String>")]
    #[schemars(length(equal = 2))]
    pub location: Option<Country>,
}
