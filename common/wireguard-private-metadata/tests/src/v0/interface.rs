// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_wireguard_private_metadata_shared::{
    v0 as latest, Construct, Extract, Request, Response, Version,
};

pub enum RequestData {
    AvailableBandwidth(()),
    TopUpBandwidth(()),
}

impl From<latest::interface::RequestData> for RequestData {
    fn from(value: latest::interface::RequestData) -> Self {
        match value {
            latest::interface::RequestData::AvailableBandwidth(inner) => {
                Self::AvailableBandwidth(inner)
            }
            latest::interface::RequestData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl From<RequestData> for latest::interface::RequestData {
    fn from(value: RequestData) -> Self {
        match value {
            RequestData::AvailableBandwidth(inner) => Self::AvailableBandwidth(inner),
            RequestData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl From<latest::interface::ResponseData> for ResponseData {
    fn from(value: latest::interface::ResponseData) -> Self {
        match value {
            latest::interface::ResponseData::AvailableBandwidth(inner) => {
                Self::AvailableBandwidth(inner)
            }
            latest::interface::ResponseData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl From<ResponseData> for latest::interface::ResponseData {
    fn from(value: ResponseData) -> Self {
        match value {
            ResponseData::AvailableBandwidth(inner) => Self::AvailableBandwidth(inner),
            ResponseData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl Construct<RequestData> for Request {
    fn construct(
        info: RequestData,
        version: Version,
    ) -> Result<Self, nym_wireguard_private_metadata_shared::ModelError> {
        match version {
            Version::V0 => {
                let translate_info = latest::interface::RequestData::from(info);
                let versioned_request =
                    latest::VersionedRequest::construct(translate_info, latest::VERSION)?;
                Ok(versioned_request.try_into()?)
            }
            _ => Err(
                nym_wireguard_private_metadata_shared::ModelError::DowngradeNotPossible {
                    from: version,
                    to: Version::V0,
                },
            ),
        }
    }
}

impl Extract<RequestData> for Request {
    fn extract(
        &self,
    ) -> Result<(RequestData, Version), nym_wireguard_private_metadata_shared::ModelError> {
        match self.version {
            Version::V0 => {
                let versioned_request = latest::VersionedRequest::try_from(self.clone())?;
                let (request, version) = versioned_request.extract()?;

                Ok((request.into(), version))
            }
            _ => Err(
                nym_wireguard_private_metadata_shared::ModelError::UpdateNotPossible {
                    from: self.version,
                    to: Version::V0,
                },
            ),
        }
    }
}

pub enum ResponseData {
    AvailableBandwidth(()),
    TopUpBandwidth(()),
}

impl Construct<ResponseData> for Response {
    fn construct(
        info: ResponseData,
        version: Version,
    ) -> Result<Self, nym_wireguard_private_metadata_shared::ModelError> {
        match version {
            Version::V0 => {
                let translate_response = latest::interface::ResponseData::from(info);
                let versioned_response =
                    latest::VersionedResponse::construct(translate_response, version)?;
                Ok(versioned_response.try_into()?)
            }
            _ => Err(
                nym_wireguard_private_metadata_shared::ModelError::DowngradeNotPossible {
                    from: version,
                    to: Version::V0,
                },
            ),
        }
    }
}

impl Extract<ResponseData> for Response {
    fn extract(
        &self,
    ) -> Result<(ResponseData, Version), nym_wireguard_private_metadata_shared::ModelError> {
        match self.version {
            Version::V0 => {
                let versioned_response = latest::VersionedResponse::try_from(self.clone())?;
                let (response, version) = versioned_response.extract()?;

                Ok((response.into(), version))
            }
            _ => Err(
                nym_wireguard_private_metadata_shared::ModelError::UpdateNotPossible {
                    from: self.version,
                    to: Version::V0,
                },
            ),
        }
    }
}
