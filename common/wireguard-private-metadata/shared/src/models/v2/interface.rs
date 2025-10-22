// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::BandwidthCredential;

use super::super::v1 as previous;

use super::{
    QueryType, VERSION, VersionedRequest, VersionedResponse,
    available_bandwidth::{
        request::InnerAvailableBandwidthRequest, response::InnerAvailableBandwidthResponse,
    },
    topup_bandwidth::{request::InnerTopUpRequest, response::InnerTopUpResponse},
};
use crate::models::{Construct, Extract, Version, error::Error};

#[derive(Debug, Clone, PartialEq)]
pub enum RequestData {
    AvailableBandwidth,
    TopUpBandwidth {
        credential: Box<BandwidthCredential>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseData {
    AvailableBandwidth { amount: i64, upgrade_mode: bool },
    TopUpBandwidth { available_bandwidth: i64 },
}

impl Construct<RequestData> for VersionedRequest {
    fn construct(info: RequestData, _version: Version) -> Result<Self, Error> {
        match info {
            RequestData::AvailableBandwidth => Ok(InnerAvailableBandwidthRequest {}.try_into()?),
            RequestData::TopUpBandwidth { credential } => Ok(InnerTopUpRequest {
                credential: *credential,
            }
            .try_into()?),
        }
    }
}

impl Extract<RequestData> for VersionedRequest {
    fn extract(&self) -> Result<(RequestData, Version), Error> {
        match self.query_type {
            QueryType::AvailableBandwidth => {
                let _req = InnerAvailableBandwidthRequest::try_from(self)?;
                Ok((RequestData::AvailableBandwidth, VERSION))
            }
            QueryType::TopUpBandwidth => {
                let req = InnerTopUpRequest::try_from(self)?;
                Ok((
                    RequestData::TopUpBandwidth {
                        credential: Box::new(req.credential),
                    },
                    VERSION,
                ))
            }
        }
    }
}

impl Construct<ResponseData> for VersionedResponse {
    fn construct(info: ResponseData, _version: Version) -> Result<Self, Error> {
        match info {
            ResponseData::AvailableBandwidth {
                amount,
                upgrade_mode,
            } => Ok(InnerAvailableBandwidthResponse {
                available_bandwidth: amount,
                upgrade_mode,
            }
            .try_into()?),
            ResponseData::TopUpBandwidth {
                available_bandwidth,
            } => Ok(InnerTopUpResponse {
                available_bandwidth,
            }
            .try_into()?),
        }
    }
}

impl Extract<ResponseData> for VersionedResponse {
    fn extract(&self) -> Result<(ResponseData, Version), Error> {
        match self.query_type {
            QueryType::AvailableBandwidth => {
                let resp = InnerAvailableBandwidthResponse::try_from(self)?;
                Ok((
                    ResponseData::AvailableBandwidth {
                        amount: resp.available_bandwidth,
                        upgrade_mode: resp.upgrade_mode,
                    },
                    VERSION,
                ))
            }
            QueryType::TopUpBandwidth => {
                let resp = InnerTopUpResponse::try_from(self)?;
                Ok((
                    ResponseData::TopUpBandwidth {
                        available_bandwidth: resp.available_bandwidth,
                    },
                    VERSION,
                ))
            }
        }
    }
}

impl TryFrom<previous::interface::RequestData> for RequestData {
    type Error = super::Error;

    fn try_from(value: previous::interface::RequestData) -> Result<Self, Self::Error> {
        match value {
            previous::interface::RequestData::AvailableBandwidth(_) => Ok(Self::AvailableBandwidth),
            previous::interface::RequestData::TopUpBandwidth(_) => {
                Err(super::Error::UpdateNotPossible {
                    from: previous::VERSION,
                    to: VERSION,
                })
            }
        }
    }
}

impl TryFrom<RequestData> for previous::interface::RequestData {
    type Error = super::Error;

    fn try_from(value: RequestData) -> Result<Self, Self::Error> {
        match value {
            RequestData::AvailableBandwidth => Ok(Self::AvailableBandwidth(())),
            RequestData::TopUpBandwidth { credential } => match *credential {
                BandwidthCredential::ZkNym(zk_nym) => Ok(Self::TopUpBandwidth(zk_nym)),
                BandwidthCredential::UpgradeModeJWT { .. } => {
                    Err(super::Error::DowngradeNotPossible {
                        from: VERSION,
                        to: previous::VERSION,
                    })
                }
            },
        }
    }
}

impl TryFrom<previous::interface::ResponseData> for ResponseData {
    type Error = super::Error;

    fn try_from(value: previous::interface::ResponseData) -> Result<Self, Self::Error> {
        match value {
            previous::interface::ResponseData::AvailableBandwidth(_) => {
                Err(super::Error::UpdateNotPossible {
                    from: previous::VERSION,
                    to: VERSION,
                })
            }
            previous::interface::ResponseData::TopUpBandwidth(amount) => {
                Ok(ResponseData::TopUpBandwidth {
                    available_bandwidth: amount,
                })
            }
        }
    }
}

impl TryFrom<ResponseData> for previous::interface::ResponseData {
    type Error = super::Error;

    fn try_from(value: ResponseData) -> Result<Self, Self::Error> {
        match value {
            ResponseData::AvailableBandwidth { amount, .. } => Ok(Self::AvailableBandwidth(amount)),
            ResponseData::TopUpBandwidth {
                available_bandwidth,
            } => Ok(Self::TopUpBandwidth(available_bandwidth)),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::models::tests::CREDENTIAL_BYTES;
    use nym_credentials_interface::CredentialSpendingData;

    use super::*;

    #[test]
    fn request_upgrade() {
        assert_eq!(
            RequestData::try_from(previous::interface::RequestData::AvailableBandwidth(()))
                .unwrap(),
            RequestData::AvailableBandwidth
        );
        assert!(
            RequestData::try_from(previous::interface::RequestData::TopUpBandwidth(Box::new(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap()
            )))
            .is_err(),
        );
    }

    #[test]
    fn response_upgrade() {
        assert!(
            ResponseData::try_from(previous::interface::ResponseData::AvailableBandwidth(42))
                .is_err()
        );
        assert_eq!(
            ResponseData::try_from(previous::interface::ResponseData::TopUpBandwidth(42)).unwrap(),
            ResponseData::TopUpBandwidth {
                available_bandwidth: 42
            }
        );
    }

    #[test]
    fn request_downgrade() {
        assert_eq!(
            previous::interface::RequestData::try_from(RequestData::AvailableBandwidth).unwrap(),
            previous::interface::RequestData::AvailableBandwidth(())
        );
        assert_eq!(
            previous::interface::RequestData::try_from(RequestData::TopUpBandwidth {
                credential: Box::new(BandwidthCredential::from(
                    CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap()
                ))
            })
            .unwrap(),
            previous::interface::RequestData::TopUpBandwidth(Box::new(
                CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap()
            ))
        );
    }

    #[test]
    fn response_downgrade() {
        assert_eq!(
            previous::interface::ResponseData::try_from(ResponseData::AvailableBandwidth {
                amount: 42,
                upgrade_mode: true
            })
            .unwrap(),
            previous::interface::ResponseData::AvailableBandwidth(42)
        );
        assert_eq!(
            previous::interface::ResponseData::try_from(ResponseData::TopUpBandwidth {
                available_bandwidth: 42
            })
            .unwrap(),
            previous::interface::ResponseData::TopUpBandwidth(42)
        );
    }
}
