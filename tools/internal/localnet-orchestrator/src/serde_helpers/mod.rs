// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, bail};
use std::net::IpAddr;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[derive(Debug)]
pub struct ContainersList {
    pub containers: Vec<CommonContainerInformation>,
}

impl ContainersList {
    pub fn new_empty() -> Self {
        ContainersList {
            containers: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ContainerInspect {
    pub info: Option<CommonContainerInformation>,
}

impl ContainerInspect {
    pub fn new_empty_container() -> ContainerInspect {
        ContainerInspect { info: None }
    }

    pub fn is_running(&self) -> bool {
        let Some(info) = &self.info else {
            return false;
        };
        info.status == "running" || info.status == "up"
    }

    pub fn container_ip(&self) -> anyhow::Result<IpAddr> {
        let Some(info) = &self.info else {
            bail!("container is not running")
        };

        info.ip_address.context("ip address not available!")
    }
}

#[derive(Debug)]
pub struct CommonContainerInformation {
    pub name: String,
    pub ip_address: Option<IpAddr>,
    pub status: String,
    pub image: String,
}
