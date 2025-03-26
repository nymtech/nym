// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::IpAddr;

pub(crate) mod network_filter;

pub(crate) trait RoutingFilter {
    fn should_route(&self, ip: IpAddr) -> bool;
}

#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct OpenFilter;

impl RoutingFilter for OpenFilter {
    fn should_route(&self, _: IpAddr) -> bool {
        true
    }
}

// #[derive(Default)]
// pub(crate) struct ComposedRoutingFilter {
//     layers: Vec<Box<dyn RoutingFilter + Send + Sync + 'static>>,
// }
// 
// impl ComposedRoutingFilter {
//     pub(crate) fn new() -> Self {
//         Self::default()
//     }
// 
//     pub(crate) fn with_filter<F: RoutingFilter + Send + Sync + 'static>(
//         mut self,
//         filter: F,
//     ) -> Self {
//         self.layers.push(Box::new(filter));
//         self
//     }
// }
// 
// impl RoutingFilter for ComposedRoutingFilter {
//     fn should_route(&self, ip: IpAddr) -> bool {
//         self.layers.iter().all(|l| l.should_route(ip))
//     }
// }
