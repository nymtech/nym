// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub use nym_contracts_common::Percent;

use crate::VpnApiClientError;

#[derive(Clone, Copy, Default, Debug)]
pub struct GatewayMinPerformance {
    pub mixnet_min_performance: Option<Percent>,
    pub vpn_min_performance: Option<Percent>,
}

impl GatewayMinPerformance {
    pub fn from_percentage_values(
        mixnet_min_performance: Option<u64>,
        vpn_min_performance: Option<u64>,
    ) -> Result<Self, VpnApiClientError> {
        let mixnet_min_performance = mixnet_min_performance
            .map(Percent::from_percentage_value)
            .transpose()
            .map_err(VpnApiClientError::InvalidPercentValue)?;
        let vpn_min_performance = vpn_min_performance
            .map(Percent::from_percentage_value)
            .transpose()
            .map_err(VpnApiClientError::InvalidPercentValue)?;
        Ok(Self {
            mixnet_min_performance,
            vpn_min_performance,
        })
    }

    pub(crate) fn to_param(self) -> Vec<(String, String)> {
        let mut params = vec![];
        if let Some(threshold) = self.mixnet_min_performance {
            params.push((
                crate::routes::MIXNET_MIN_PERFORMANCE.to_string(),
                threshold.to_string(),
            ));
        };
        if let Some(threshold) = self.vpn_min_performance {
            params.push((
                crate::routes::VPN_MIN_PERFORMANCE.to_string(),
                threshold.to_string(),
            ));
        };
        params
    }
}

#[derive(Clone, Debug)]
pub enum GatewayType {
    MixnetEntry,
    MixnetExit,
    Wg,
}

#[derive(Clone, Copy, Debug)]
pub struct ScoreThresholds {
    pub high: u8,
    pub medium: u8,
    pub low: u8,
}
