// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credential_verification::upgrade_mode::UpgradeModeDetails;
use nym_credentials_interface::BandwidthCredential;
use std::cmp::max;
use std::net::IpAddr;

use crate::transceiver::PeerControllerTransceiver;
use nym_wireguard_private_metadata_shared::error::MetadataError;
use nym_wireguard_private_metadata_shared::interface::ResponseData;

// we need to be above MINIMUM_REMAINING_BANDWIDTH (500MB) plus we also have to trick the client
// its depletion is low enough to not require sending new tickets
const DEFAULT_WG_CLIENT_BANDWIDTH_THRESHOLD: i64 = 1024 * 1024 * 1024;

#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
    transceiver: PeerControllerTransceiver,
    #[from_ref(skip)]
    upgrade_mode: UpgradeModeDetails,
}

impl AppState {
    pub fn new(transceiver: PeerControllerTransceiver, upgrade_mode: UpgradeModeDetails) -> Self {
        Self {
            transceiver,
            upgrade_mode,
        }
    }

    fn upgrade_mode_bandwidth(&self, true_bandwidth: i64) -> i64 {
        // if we're undergoing upgrade mode, we don't meter bandwidth,
        // we simply return MAX of clients current bandwidth and minimum bandwidth before default
        // client would have attempted to send new ticket (hopefully)
        // the latter is to support older clients that will ignore `upgrade_mode` field in the response
        // as they're not aware of its existence
        max(DEFAULT_WG_CLIENT_BANDWIDTH_THRESHOLD, true_bandwidth)
    }

    pub async fn available_bandwidth(&self, ip: IpAddr) -> Result<ResponseData, MetadataError> {
        let upgrade_mode = self.upgrade_mode.enabled();

        let true_bandwidth = self.transceiver.query_bandwidth(ip).await?;
        let available_bandwidth = if upgrade_mode {
            self.upgrade_mode_bandwidth(true_bandwidth)
        } else {
            true_bandwidth
        };

        Ok(ResponseData::AvailableBandwidth {
            amount: available_bandwidth,
            upgrade_mode,
        })
    }

    // Top up with a credential and return the afterwards available bandwidth
    pub async fn topup_bandwidth(
        &self,
        ip: IpAddr,
        claim: Box<BandwidthCredential>,
    ) -> Result<ResponseData, MetadataError> {
        match *claim {
            BandwidthCredential::ZkNym(zk_nym) => {
                // if we got zk-nym, we just try to verify it
                let available_bandwidth = self.transceiver.topup_bandwidth(ip, zk_nym).await?;

                // however, we still follow the same upgrade-mode logic,
                // so that the client would not attempt to needlessly send more credentials
                let upgrade_mode = self.upgrade_mode.enabled();
                let available_bandwidth = if upgrade_mode {
                    self.upgrade_mode_bandwidth(available_bandwidth)
                } else {
                    available_bandwidth
                };

                Ok(ResponseData::TopUpBandwidth {
                    available_bandwidth,
                    upgrade_mode,
                })
            }
            BandwidthCredential::UpgradeModeJWT { token } => {
                // if we're already in the upgrade mode, don't bother validating the token
                if self.upgrade_mode.enabled() {
                    let true_bandwidth = self.transceiver.query_bandwidth(ip).await?;
                    return Ok(ResponseData::TopUpBandwidth {
                        available_bandwidth: self.upgrade_mode_bandwidth(true_bandwidth),
                        upgrade_mode: true,
                    });
                }

                // if the token is valid, try to check if we're behind
                // and have to update our internal state
                self.upgrade_mode
                    .try_enable_via_received_jwt(token)
                    .await
                    .map_err(|err| MetadataError::JWTVerification {
                        message: err.to_string(),
                    })?;

                // if we didn't return an error, it means token got accepted
                // and we have transitioned into the upgrade mode
                let true_bandwidth = self.transceiver.query_bandwidth(ip).await?;

                Ok(ResponseData::TopUpBandwidth {
                    available_bandwidth: self.upgrade_mode_bandwidth(true_bandwidth),
                    upgrade_mode: true,
                })
            }
        }
    }
}
