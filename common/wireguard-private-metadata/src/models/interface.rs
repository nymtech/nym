// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::CredentialSpendingData;

#[cfg(test)]
use crate::models::v0;
use crate::models::{v1, Construct, Extract, Request, Response, Version};

pub enum RequestData {
    AvailableBandwidth(()),
    TopUpBandwidth(CredentialSpendingData),
}

impl From<super::latest::interface::RequestData> for RequestData {
    fn from(value: super::latest::interface::RequestData) -> Self {
        match value {
            super::latest::interface::RequestData::AvailableBandwidth(inner) => {
                Self::AvailableBandwidth(inner)
            }
            super::latest::interface::RequestData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl From<RequestData> for super::latest::interface::RequestData {
    fn from(value: RequestData) -> Self {
        match value {
            RequestData::AvailableBandwidth(inner) => Self::AvailableBandwidth(inner),
            RequestData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl From<super::latest::interface::ResponseData> for ResponseData {
    fn from(value: super::latest::interface::ResponseData) -> Self {
        match value {
            super::latest::interface::ResponseData::AvailableBandwidth(inner) => {
                Self::AvailableBandwidth(inner)
            }
            super::latest::interface::ResponseData::TopUpBandwidth(credential_spending_data) => {
                Self::TopUpBandwidth(credential_spending_data)
            }
        }
    }
}

impl From<ResponseData> for super::latest::interface::ResponseData {
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
    fn construct(info: RequestData, version: Version) -> Result<Self, super::error::Error> {
        match version {
            #[cfg(test)]
            Version::V0 => todo!(),
            Version::V1 => {
                let versioned_request = v1::VersionedRequest::construct(info.into(), version)?;
                Ok(versioned_request.try_into()?)
            }
        }
    }
}

impl Extract<RequestData> for Request {
    fn extract(&self) -> Result<(RequestData, Version), crate::models::Error> {
        match self.version {
            #[cfg(test)]
            super::Version::V0 => {
                let versioned_request = v0::VersionedRequest::try_from(self.clone())?;
                let (request, version) = versioned_request.extract()?;

                let upgrade_request = super::latest::interface::RequestData::try_from(request)?;

                Ok((upgrade_request.into(), version))
            }
            super::Version::V1 => {
                let versioned_request = v1::VersionedRequest::try_from(self.clone())?;
                let (extracted, version) = versioned_request.extract()?;
                Ok((extracted.into(), version))
            }
        }
    }
}

pub enum ResponseData {
    AvailableBandwidth(i64),
    TopUpBandwidth(i64),
}

impl Construct<ResponseData> for Response {
    fn construct(info: ResponseData, version: Version) -> Result<Self, super::error::Error> {
        match version {
            #[cfg(test)]
            super::Version::V0 => {
                let translate_response = super::latest::interface::ResponseData::from(info);
                let downgrade_response = v0::interface::ResponseData::try_from(translate_response)?;
                let versioned_response =
                    v0::VersionedResponse::construct(downgrade_response, version)?;
                Ok(versioned_response.try_into()?)
            }
            Version::V1 => {
                let versioned_response = v1::VersionedResponse::construct(info.into(), version)?;
                Ok(versioned_response.try_into()?)
            }
        }
    }
}

impl Extract<ResponseData> for Response {
    fn extract(&self) -> Result<(ResponseData, Version), super::error::Error> {
        match self.version {
            #[cfg(test)]
            super::Version::V0 => {
                let versioned_response = v0::VersionedResponse::try_from(self.clone())?;
                let (response, version) = versioned_response.extract()?;

                let upgrade_response = super::latest::interface::ResponseData::try_from(response)?;

                Ok((upgrade_response.into(), version))
            }
            super::Version::V1 => {
                let versioned_response = v1::VersionedResponse::try_from(self.clone())?;
                let (extracted, version) = versioned_response.extract()?;
                Ok((extracted.into(), version))
            }
        }
    }
}
