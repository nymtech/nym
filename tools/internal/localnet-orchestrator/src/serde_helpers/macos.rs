// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTAINER_NETWORK_NAME;
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainersList(Vec<ContainerInformation>);

impl TryFrom<ContainersList> for super::ContainersList {
    type Error = anyhow::Error;

    fn try_from(value: ContainersList) -> Result<Self, Self::Error> {
        Ok(super::ContainersList {
            containers: value
                .0
                .into_iter()
                .filter(|c| c.configuration.id.contains("localnet"))
                .map(TryInto::try_into)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerInspect(pub(crate) Vec<ContainerInformation>);

impl TryFrom<ContainerInspect> for super::ContainerInspect {
    type Error = anyhow::Error;

    fn try_from(mut value: ContainerInspect) -> Result<Self, Self::Error> {
        if value.0.is_empty() {
            return Ok(super::ContainerInspect::new_empty_container());
        }

        if value.0.len() != 1 {
            bail!("more than a single container information")
        }

        // SAFETY: we just checked we have exactly one element
        #[allow(clippy::unwrap_used)]
        let info = value.0.pop().unwrap();
        Ok(super::ContainerInspect {
            info: Some(info.try_into()?),
        })
    }
}

// we only care about a tiny subset of fields
pub mod container_network_inspect {
    use serde::{Deserialize, Serialize};
    use std::net::IpAddr;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct NetworkInspect(Vec<NetworkInspectInner>);

    impl NetworkInspect {
        pub fn is_running(&self) -> bool {
            let Some(inner) = &self.0.first() else {
                return false;
            };
            inner.state == "running"
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct NetworkInspectInner {
        pub config: Config,
        pub status: Status,
        pub state: String,
        pub id: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Config {
        pub id: String,
        pub mode: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Status {
        pub address: String, // represented in cidr location
        pub gateway: IpAddr,
    }
}

// note: this contains only a small subset of possible fields
#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerInformation {
    pub status: String,
    pub configuration: ContainerConfiguration,
    pub networks: Vec<ContainerNetwork>,
}

impl ContainerInformation {
    pub fn container_ip(&self) -> anyhow::Result<IpAddr> {
        for network in &self.networks {
            if network.network == CONTAINER_NETWORK_NAME {
                // perform the split in case the network is provided in cidr notation
                let raw_address = network
                    .address
                    .split('/')
                    .next()
                    .unwrap_or(&network.address);

                return raw_address.parse().context("malformed network ip address");
            }
        }

        bail!(
            "no container ip address found. full network information: {:#?}",
            self.networks
        )
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerConfiguration {
    pub id: String,
    pub image: ContainerImage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerImage {
    // e.g. "docker.io/library/localnet-nym-binaries:1.22.0"
    pub reference: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerNetwork {
    pub hostname: String,
    pub network: String,
    pub gateway: IpAddr,
    pub address: String, // represented in cidr location
}

impl TryFrom<ContainerInformation> for super::CommonContainerInformation {
    type Error = anyhow::Error;

    #[track_caller]
    fn try_from(value: ContainerInformation) -> Result<Self, Self::Error> {
        let status = value.status.to_lowercase();
        let ip_address = if status == "running" || status == "up" {
            Some(value.container_ip().context(format!(
                "invalid container {} ({})",
                value.configuration.id, value.configuration.image.reference
            ))?)
        } else {
            None
        };

        Ok(super::CommonContainerInformation {
            ip_address,
            name: value.configuration.id,
            image: value.configuration.image.reference,
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_container_inspect_response_parsing() {
        let raw = r#"
[
  {
    "networks": [
      {
        "network": "nym-localnet",
        "gateway": "192.168.64.1",
        "hostname": "test2",
        "address": "192.168.64.65/24"
      }
    ],
    "configuration": {
      "publishedPorts": [],
      "publishedSockets": [],
      "dns": {
        "searchDomains": [],
        "options": [],
        "nameservers": []
      },
      "image": {
        "descriptor": {
          "mediaType": "application/vnd.oci.image.index.v1+json",
          "digest": "sha256:448b70986d8b75d3d2d465c856e6cd861c6df92263cab8a8b8350d7eea717529",
          "size": 856,
          "annotations": {
            "org.opencontainers.image.ref.name": "1.22.0",
            "io.containerd.image.name": "docker.io/library/localnet-nym-binaries:1.22.0"
          }
        },
        "reference": "docker.io/library/localnet-nym-binaries:1.22.0"
      },
      "virtualization": false,
      "mounts": [],
      "rosetta": true,
      "labels": {},
      "initProcess": {
        "user": {
          "id": {
            "uid": 0,
            "gid": 0
          }
        },
        "arguments": [],
        "workingDirectory": "/nym",
        "environment": [
          "PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
          "RUSTUP_HOME=/usr/local/rustup",
          "CARGO_HOME=/usr/local/cargo",
          "RUST_VERSION=1.91.1"
        ],
        "executable": "sh",
        "supplementalGroups": [],
        "rlimits": [],
        "terminal": true
      },
      "sysctls": {},
      "runtimeHandler": "container-runtime-linux",
      "platform": {
        "architecture": "amd64",
        "os": "linux"
      },
      "networks": [
        {
          "network": "nym-localnet",
          "options": {
            "hostname": "test2"
          }
        }
      ],
      "ssh": false,
      "id": "test2",
      "resources": {
        "cpus": 4,
        "memoryInBytes": 1073741824
      }
    },
    "status": "running"
  }
]
        "#;

        let parsed: ContainerInspect = serde_json::from_str(raw).unwrap();
        let inner = parsed.0.first().unwrap();
        assert_eq!(inner.configuration.id, "test2");
        assert_eq!(inner.networks[0].address, "192.168.64.65/24");
    }
}
