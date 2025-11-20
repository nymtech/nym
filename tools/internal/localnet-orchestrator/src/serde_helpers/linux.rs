// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod container_network_inspect {
    use serde::{Deserialize, Serialize};
    use std::net::IpAddr;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct NetworkInspect(pub(crate) Vec<NetworkInspectInner>);

    impl NetworkInspect {
        // not sure if it's the best test
        // but given existing schema, couldn't think of anything better
        pub fn is_running(&self) -> bool {
            let Some(inner) = &self.0.first() else {
                return false;
            };
            // check we actually have defined subnet with a gateway
            !inner.ipam.config.is_empty()
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    pub struct NetworkInspectInner {
        pub name: String,
        pub id: String,
        #[serde(alias = "IPAM")]
        pub ipam: Ipam,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    pub struct Ipam {
        pub config: Vec<IpamConfig>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    pub struct IpamConfig {
        pub subnet: String, // represented in cidr location
        pub gateway: IpAddr,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ContainersList(pub(crate) Vec<ContainerListContainer>);

impl TryFrom<ContainersList> for super::ContainersList {
    type Error = anyhow::Error;

    fn try_from(value: ContainersList) -> Result<Self, Self::Error> {
        Ok(super::ContainersList {
            containers: value
                .0
                .into_iter()
                .filter(|c| c.names.contains("localnet"))
                .map(TryInto::try_into)
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerListContainer {
    pub command: String,

    #[serde(alias = "ID")]
    pub id: String,

    pub image: String,
    pub names: String,
    pub status: String,
}

impl TryFrom<ContainerListContainer> for super::CommonContainerInformation {
    type Error = anyhow::Error;

    fn try_from(value: ContainerListContainer) -> Result<Self, Self::Error> {
        Ok(super::CommonContainerInformation {
            name: value.names,
            ip_address: None,
            status: value.status.to_lowercase(),
            image: value.image,
        })
    }
}

// note: this contains only a small subset of possible fields
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInformation {
    pub id: String,
    pub state: State,
    pub image: String,
    pub name: String,
    pub network_settings: NetworkSettings,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct State {
    pub status: String,
    pub running: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkSettings {
    pub mac_address: String,
    pub networks: HashMap<String, ContainerNetworkInformation>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerNetworkInformation {
    #[serde(alias = "IPAddress")]
    pub ip_address: String,

    #[serde(alias = "IPPrefixLen")]
    pub ip_prefix_len: u8,

    #[serde(alias = "GlobalIPv6Address")]
    pub global_ipv6_address: String,

    #[serde(alias = "GlobalIPv6PrefixLen")]
    pub global_ipv6_prefix_len: u8,

    pub mac_address: String,
}

impl TryFrom<ContainerInformation> for super::CommonContainerInformation {
    type Error = anyhow::Error;

    fn try_from(value: ContainerInformation) -> Result<Self, Self::Error> {
        let status = value.state.status.to_lowercase();

        let ip_address = if status == "running" || status == "up" {
            if value.network_settings.networks.is_empty() {
                bail!("no attached networks")
            }

            // find first network with non-empty ip address
            let Some(network) = value
                .network_settings
                .networks
                .iter()
                .find(|n| !n.1.ip_address.is_empty())
                .map(|n| n.1)
            else {
                bail!("no valid network")
            };
            Some(network.ip_address.parse().context("invalid ip address")?)
        } else {
            None
        };

        Ok(super::CommonContainerInformation {
            name: value.name,
            image: value.image,
            ip_address,
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::container_network_inspect::NetworkInspect;
    use crate::serde_helpers::linux::ContainerInspect;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn sample_network_inspect_response_parsing() {
        let raw = r#"
[
    {
        "Name": "test",
        "Id": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "IPAM": {
            "Config": [
                {
                    "Subnet": "10.4.2.0/24",
                    "Gateway": "10.4.2.1"
                }
            ]
        },
        "Labels": {},
        "Containers": {}
    }
]
        "#;

        let parsed: NetworkInspect = serde_json::from_str(raw).unwrap();
        let inner = parsed.0.first().unwrap();
        assert_eq!(inner.name, "test");
        assert_eq!(inner.ipam.config.first().unwrap().subnet, "10.4.2.0/24")
    }

    #[test]
    fn sample_container_inspect_response_parsing() {
        let raw = r#"
  [
    {
        "Id": "22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b",
        "Created": "2025-12-04T17:18:57.561222924Z",
        "Path": "sleep",
        "Args": [
            "1000"
        ],
        "State": {
            "Status": "running",
            "Running": true,
            "Paused": false,
            "Restarting": false,
            "Pid": 101647,
            "ExitCode": 0,
            "Error": "",
            "StartedAt": "2025-12-04T17:18:57.806684779Z",
            "FinishedAt": ""
        },
        "Image": "docker.io/library/localnet-nyxd:v0.60.1",
        "ResolvConfPath": "/var/lib/nerdctl/1935db59/containers/default/22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b/resolv.conf",
        "HostnamePath": "/var/lib/nerdctl/1935db59/containers/default/22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b/hostname",
        "HostsPath": "/var/lib/nerdctl/1935db59/etchosts/default/22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b/hosts",
        "LogPath": "/var/lib/nerdctl/1935db59/containers/default/22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b/22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b-json.log",
        "Name": "lab-nature-localnet-nyxdab",
        "RestartCount": 0,
        "Driver": "overlayfs",
        "Platform": "linux",
        "AppArmorProfile": "nerdctl-default",
        "HostConfig": {
            "ContainerIDFile": "",
            "LogConfig": {
                "driver": "json-file",
                "address": "/run/containerd/containerd.sock"
            },
            "PortBindings": {},
            "CgroupnsMode": "private",
            "Dns": null,
            "DnsOptions": null,
            "DnsSearch": null,
            "ExtraHosts": [],
            "GroupAdd": [
                "1",
                "2",
                "3",
                "4",
                "6",
                "10",
                "11",
                "20",
                "26",
                "27"
            ],
            "IpcMode": "private",
            "OomScoreAdj": 0,
            "PidMode": "",
            "ReadonlyRootfs": false,
            "UTSMode": "",
            "ShmSize": 0,
            "Sysctls": null,
            "Runtime": "io.containerd.runc.v2",
            "CpusetMems": "",
            "CpusetCpus": "",
            "CpuQuota": 0,
            "CpuShares": 0,
            "CpuPeriod": 0,
            "CpuRealtimePeriod": 0,
            "CpuRealtimeRuntime": 0,
            "Memory": 0,
            "MemorySwap": 0,
            "OomKillDisable": false,
            "Devices": null,
            "BlkioWeight": 0,
            "BlkioWeightDevice": [],
            "BlkioDeviceReadBps": [],
            "BlkioDeviceWriteBps": [],
            "BlkioDeviceReadIOps": [],
            "BlkioDeviceWriteIOps": []
        },
        "Mounts": [
            {
                "Type": "bind",
                "Source": "/root/.nym/localnet-orchestrator/lab-nature/nyxd",
                "Destination": "/root/.nyxd",
                "Mode": "",
                "RW": true,
                "Propagation": ""
            }
        ],
        "Config": {
            "Hostname": "22a611624245",
            "AttachStdin": false,
            "Env": [
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "HOSTNAME=22a611624245"
            ],
            "Image": "docker.io/library/localnet-nyxd:v0.60.1",
            "Labels": {
                "io.containerd.image.config.stop-signal": "SIGTERM",
                "nerdctl/auto-remove": "false",
                "nerdctl/dns": "{\"DNSServers\":null,\"DNSResolvConfOptions\":null,\"DNSSearchDomains\":null}",
                "nerdctl/extraHosts": "[]",
                "nerdctl/host-config": "{\"BlkioWeight\":0,\"CidFile\":\"\",\"Devices\":null}",
                "nerdctl/hostname": "22a611624245",
                "nerdctl/ipc": "{\"mode\":\"private\"}",
                "nerdctl/log-config": "{\"driver\":\"json-file\",\"address\":\"/run/containerd/containerd.sock\"}",
                "nerdctl/log-uri": "binary:///usr/local/bin/nerdctl?_NERDCTL_INTERNAL_LOGGING=%2Fvar%2Flib%2Fnerdctl%2F1935db59",
                "nerdctl/mounts": "[{\"Type\":\"bind\",\"Source\":\"/root/.nym/localnet-orchestrator/lab-nature/nyxd\",\"Destination\":\"/root/.nyxd\",\"Mode\":\"\",\"RW\":true,\"Propagation\":\"\"}]",
                "nerdctl/name": "lab-nature-localnet-nyxdab",
                "nerdctl/namespace": "default",
                "nerdctl/networks": "[\"nym-localnet\"]",
                "nerdctl/platform": "linux/amd64",
                "nerdctl/state-dir": "/var/lib/nerdctl/1935db59/containers/default/22a611624245e35fee8a15126f23f2b226d8ea3800a213823605303f4988183b"
            }
        },
        "NetworkSettings": {
            "Ports": {},
            "GlobalIPv6Address": "",
            "GlobalIPv6PrefixLen": 0,
            "IPAddress": "10.4.1.2",
            "IPPrefixLen": 24,
            "MacAddress": "3a:74:44:fc:cf:d2",
            "Networks": {
                "unknown-eth0": {
                    "IPAddress": "10.4.1.2",
                    "IPPrefixLen": 24,
                    "GlobalIPv6Address": "",
                    "GlobalIPv6PrefixLen": 0,
                    "MacAddress": "3a:74:44:fc:cf:d2"
                }
            }
        }
    }
]"#;

        let parsed: ContainerInspect = serde_json::from_str(raw).unwrap();
        let inner = parsed.0.first().unwrap();
        assert_eq!(inner.name, "lab-nature-localnet-nyxdab");
        assert_eq!(
            inner.network_settings.networks["unknown-eth0"].ip_address,
            "10.4.1.2"
        );

        let another_raw = r#"
[
    {
        "Id": "1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1",
        "Created": "2025-12-05T21:53:08.721912948Z",
        "Path": "nyxd",
        "Args": [
            "start"
        ],
        "State": {
            "Status": "running",
            "Running": true,
            "Paused": false,
            "Restarting": false,
            "Pid": 80138,
            "ExitCode": 0,
            "Error": "",
            "StartedAt": "2025-12-05T21:53:09.212670473Z",
            "FinishedAt": ""
        },
        "Image": "docker.io/library/localnet-nyxd:v0.60.1",
        "ResolvConfPath": "/var/lib/nerdctl/1935db59/containers/default/1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1/resolv.conf",
        "HostnamePath": "/var/lib/nerdctl/1935db59/containers/default/1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1/hostname",
        "HostsPath": "/var/lib/nerdctl/1935db59/etchosts/default/1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1/hosts",
        "LogPath": "/var/lib/nerdctl/1935db59/containers/default/1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1/1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1-json.log",
        "Name": "minimum-fatal-localnet-nyxd",
        "RestartCount": 0,
        "Driver": "overlayfs",
        "Platform": "linux",
        "AppArmorProfile": "nerdctl-default",
        "HostConfig": {
            "ContainerIDFile": "",
            "LogConfig": {
                "driver": "json-file",
                "address": "/run/containerd/containerd.sock"
            },
            "PortBindings": {
                "26657/tcp": [
                    {
                        "HostIp": "0.0.0.0",
                        "HostPort": "26657"
                    }
                ]
            },
            "CgroupnsMode": "private",
            "Dns": null,
            "DnsOptions": null,
            "DnsSearch": null,
            "ExtraHosts": [],
            "GroupAdd": [
                "1",
                "2",
                "3",
                "4",
                "6",
                "10",
                "11",
                "20",
                "26",
                "27"
            ],
            "IpcMode": "private",
            "OomScoreAdj": 0,
            "PidMode": "",
            "ReadonlyRootfs": false,
            "UTSMode": "",
            "ShmSize": 0,
            "Sysctls": null,
            "Runtime": "io.containerd.kata.v2",
            "CpusetMems": "",
            "CpusetCpus": "",
            "CpuQuota": 0,
            "CpuShares": 0,
            "CpuPeriod": 0,
            "CpuRealtimePeriod": 0,
            "CpuRealtimeRuntime": 0,
            "Memory": 0,
            "MemorySwap": 0,
            "OomKillDisable": false,
            "Devices": null,
            "BlkioWeight": 0,
            "BlkioWeightDevice": [],
            "BlkioDeviceReadBps": [],
            "BlkioDeviceWriteBps": [],
            "BlkioDeviceReadIOps": [],
            "BlkioDeviceWriteIOps": []
        },
        "Mounts": [
            {
                "Type": "bind",
                "Source": "/root/.nym/localnet-orchestrator/minimum-fatal/nyxd",
                "Destination": "/root/.nyxd",
                "Mode": "",
                "RW": true,
                "Propagation": ""
            }
        ],
        "Config": {
            "Hostname": "1de89e6c7815",
            "AttachStdin": false,
            "Env": [
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "HOSTNAME=1de89e6c7815"
            ],
            "Image": "docker.io/library/localnet-nyxd:v0.60.1",
            "Labels": {
                "io.containerd.image.config.stop-signal": "SIGTERM",
                "nerdctl/auto-remove": "false",
                "nerdctl/dns": "{\"DNSServers\":null,\"DNSResolvConfOptions\":null,\"DNSSearchDomains\":null}",
                "nerdctl/extraHosts": "[]",
                "nerdctl/host-config": "{\"BlkioWeight\":0,\"CidFile\":\"\",\"Devices\":null}",
                "nerdctl/hostname": "1de89e6c7815",
                "nerdctl/ipc": "{\"mode\":\"private\"}",
                "nerdctl/log-config": "{\"driver\":\"json-file\",\"address\":\"/run/containerd/containerd.sock\"}",
                "nerdctl/log-uri": "binary:///usr/local/bin/nerdctl?_NERDCTL_INTERNAL_LOGGING=%2Fvar%2Flib%2Fnerdctl%2F1935db59",
                "nerdctl/mounts": "[{\"Type\":\"bind\",\"Source\":\"/root/.nym/localnet-orchestrator/minimum-fatal/nyxd\",\"Destination\":\"/root/.nyxd\",\"Mode\":\"\",\"RW\":true,\"Propagation\":\"\"}]",
                "nerdctl/name": "minimum-fatal-localnet-nyxd",
                "nerdctl/namespace": "default",
                "nerdctl/networks": "[\"nym-localnet\"]",
                "nerdctl/platform": "linux/amd64",
                "nerdctl/state-dir": "/var/lib/nerdctl/1935db59/containers/default/1de89e6c7815894e74155922cb4c4fd0524b0809000bb84e0ef5e0d98a8d7ed1"
            }
        },
        "NetworkSettings": {
            "Ports": {
                "26657/tcp": [
                    {
                        "HostIp": "0.0.0.0",
                        "HostPort": "26657"
                    }
                ]
            },
            "GlobalIPv6Address": "",
            "GlobalIPv6PrefixLen": 0,
            "IPAddress": "10.4.1.19",
            "IPPrefixLen": 24,
            "MacAddress": "0a:5c:e1:01:0a:ee",
            "Networks": {
                "unknown-eth0": {
                    "IPAddress": "10.4.1.19",
                    "IPPrefixLen": 24,
                    "GlobalIPv6Address": "",
                    "GlobalIPv6PrefixLen": 0,
                    "MacAddress": "0a:5c:e1:01:0a:ee"
                },
                "unknown-tap0_kata": {
                    "IPAddress": "",
                    "IPPrefixLen": 0,
                    "GlobalIPv6Address": "",
                    "GlobalIPv6PrefixLen": 0,
                    "MacAddress": "4a:84:9d:af:d0:a6"
                }
            }
        }
    }
]

"#;

        let parsed: ContainerInspect = serde_json::from_str(another_raw).unwrap();
        let inner = parsed.0.first().unwrap();
        assert_eq!(inner.name, "minimum-fatal-localnet-nyxd");
        assert_eq!(
            inner.network_settings.networks["unknown-eth0"].ip_address,
            "10.4.1.19"
        );
    }
}
