// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GatewayRequestsError;
use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use nym_credentials_interface::{CoconutError, VerifyCredentialRequest};
use serde::{Deserialize, Serialize};

// reimplements old coconut-interface::Credential for backwards compatibility sake
// (so that 'new' gateways could still understand those requests)
#[derive(Debug, PartialEq, Eq)]
pub struct OldV1Credential {
    pub n_params: u32,

    pub theta: VerifyCredentialRequest,

    pub voucher_value: u64,

    pub voucher_info: String,

    pub epoch_id: u64,
}

// attempt to convert the old request type into the new variant
impl TryFrom<OldV1Credential> for CredentialSpendingRequest {
    type Error = GatewayRequestsError;

    fn try_from(value: OldV1Credential) -> Result<Self, Self::Error> {
        if value.n_params <= 2 {
            return Err(GatewayRequestsError::InvalidNumberOfEmbededParameters(
                value.n_params,
            ));
        }
        let embedded_private_attributes = value.n_params as usize - 2;
        let typ = value.voucher_info.parse()?;
        let public_attributes_plain = vec![value.voucher_value.to_string(), value.voucher_info];

        Ok(CredentialSpendingRequest {
            data: CredentialSpendingData {
                embedded_private_attributes,
                verify_credential_request: value.theta,
                public_attributes_plain,
                typ,
                epoch_id: value.epoch_id,
            },
        })
    }
}

impl OldV1Credential {
    pub fn as_bytes(&self) -> Vec<u8> {
        let n_params_bytes = self.n_params.to_be_bytes();
        let theta_bytes = self.theta.to_bytes();
        let theta_bytes_len = theta_bytes.len();
        let voucher_value_bytes = self.voucher_value.to_be_bytes();
        let epoch_id_bytes = self.epoch_id.to_be_bytes();
        let voucher_info_bytes = self.voucher_info.as_bytes();
        let voucher_info_len = voucher_info_bytes.len();

        let mut bytes = Vec::with_capacity(28 + theta_bytes_len + voucher_info_len);
        bytes.extend_from_slice(&n_params_bytes);
        bytes.extend_from_slice(&(theta_bytes_len as u64).to_be_bytes());
        bytes.extend_from_slice(&theta_bytes);
        bytes.extend_from_slice(&voucher_value_bytes);
        bytes.extend_from_slice(&epoch_id_bytes);
        bytes.extend_from_slice(voucher_info_bytes);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoconutError> {
        if bytes.len() < 28 {
            return Err(CoconutError::Deserialization(String::from(
                "To few bytes in credential",
            )));
        }
        let mut four_byte = [0u8; 4];
        let mut eight_byte = [0u8; 8];

        four_byte.copy_from_slice(&bytes[..4]);
        let n_params = u32::from_be_bytes(four_byte);
        eight_byte.copy_from_slice(&bytes[4..12]);
        let theta_len = u64::from_be_bytes(eight_byte);
        if bytes.len() < 28 + theta_len as usize {
            return Err(CoconutError::Deserialization(String::from(
                "To few bytes in credential",
            )));
        }
        let theta = VerifyCredentialRequest::from_bytes(&bytes[12..12 + theta_len as usize])
            .map_err(|e| CoconutError::Deserialization(e.to_string()))?;
        eight_byte.copy_from_slice(&bytes[12 + theta_len as usize..20 + theta_len as usize]);
        let voucher_value = u64::from_be_bytes(eight_byte);
        eight_byte.copy_from_slice(&bytes[20 + theta_len as usize..28 + theta_len as usize]);
        let epoch_id = u64::from_be_bytes(eight_byte);
        let voucher_info = String::from_utf8(bytes[28 + theta_len as usize..].to_vec())
            .map_err(|e| CoconutError::Deserialization(e.to_string()))?;

        Ok(OldV1Credential {
            n_params,
            theta,
            voucher_value,
            voucher_info,
            epoch_id,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialSpendingRequest {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,
}

// just a helper macro for checking required length and advancing the buffer
macro_rules! ensure_len_and_advance {
    ($b:expr, $n:expr) => {{
        if $b.len() < $n {
            return Err(GatewayRequestsError::CredentialDeserializationFailureEOF);
        }
        // create binding to the desired range
        let bytes = &$b[..$n];

        // update the initial binding

        $b = &$b[$n..];

        bytes
    }};
}

impl CredentialSpendingRequest {
    pub fn new(data: CredentialSpendingData) -> Self {
        CredentialSpendingRequest { data }
    }

    pub fn matches_blinded_serial_number(
        &self,
        blinded_serial_number_bs58: &str,
    ) -> Result<bool, CoconutError> {
        self.data
            .verify_credential_request
            .has_blinded_serial_number(blinded_serial_number_bs58)
    }

    pub fn unchecked_voucher_value(&self) -> u64 {
        self.data
            .get_bandwidth_attribute()
            .expect("failed to extract bandwidth attribute")
            .parse()
            .expect("failed to parse voucher value")
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // simple length prefixed serialization
        // TODO: change it to a standard format instead
        let mut bytes = Vec::new();

        let embedded_private = (self.data.embedded_private_attributes as u32).to_be_bytes();
        let theta = self.data.verify_credential_request.to_bytes();
        let theta_len = (theta.len() as u32).to_be_bytes();

        let public = (self.data.public_attributes_plain.len() as u32).to_be_bytes();
        let typ = self.data.typ.to_string();
        let typ_bytes = typ.as_bytes();
        let typ_len = (typ_bytes.len() as u32).to_be_bytes();

        bytes.extend_from_slice(&embedded_private);
        bytes.extend_from_slice(&theta_len);
        bytes.extend_from_slice(&theta);
        bytes.extend_from_slice(&public);

        for pub_element in &self.data.public_attributes_plain {
            let bytes_el = pub_element.as_bytes();
            let len = (bytes_el.len() as u32).to_be_bytes();

            bytes.extend_from_slice(&len);
            bytes.extend_from_slice(bytes_el);
        }

        bytes.extend_from_slice(&typ_len);
        bytes.extend_from_slice(typ_bytes);
        bytes.extend_from_slice(&self.data.epoch_id.to_be_bytes());

        bytes
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, GatewayRequestsError> {
        // initial binding
        let mut b = raw;
        let embedded_private_bytes = ensure_len_and_advance!(b, 4);
        let embedded_private_attributes =
            u32::from_be_bytes(embedded_private_bytes.try_into().unwrap()) as usize;

        let theta_len_bytes = ensure_len_and_advance!(b, 4);
        let theta_len = u32::from_be_bytes(theta_len_bytes.try_into().unwrap()) as usize;

        let theta_bytes = ensure_len_and_advance!(b, theta_len);
        let theta = VerifyCredentialRequest::from_bytes(theta_bytes)
            .map_err(GatewayRequestsError::CredentialDeserializationFailureMalformedTheta)?;

        let public_bytes = ensure_len_and_advance!(b, 4);
        let public = u32::from_be_bytes(public_bytes.try_into().unwrap()) as usize;

        let mut public_attributes_plain = Vec::with_capacity(public);
        for _ in 0..public {
            let element_len_bytes = ensure_len_and_advance!(b, 4);
            let element_len = u32::from_be_bytes(element_len_bytes.try_into().unwrap()) as usize;

            let element_bytes = ensure_len_and_advance!(b, element_len);
            let element = String::from_utf8(element_bytes.to_vec())?;
            public_attributes_plain.push(element);
        }

        let typ_len_bytes = ensure_len_and_advance!(b, 4);
        let typ_len = u32::from_be_bytes(typ_len_bytes.try_into().unwrap()) as usize;

        let typ_bytes = ensure_len_and_advance!(b, typ_len);
        let raw_typ = String::from_utf8(typ_bytes.to_vec())?;
        let typ = raw_typ.parse()?;

        // tell the linter to chill out in for this last iteration
        #[allow(unused_assignments)]
        let epoch_id_bytes = ensure_len_and_advance!(b, 8);
        let epoch_id = u64::from_be_bytes(epoch_id_bytes.try_into().unwrap());

        Ok(CredentialSpendingRequest {
            data: CredentialSpendingData {
                embedded_private_attributes,
                verify_credential_request: theta,
                public_attributes_plain,
                typ,
                epoch_id,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_credentials::coconut::bandwidth::bandwidth_credential_params;
    use nym_credentials::IssuanceBandwidthCredential;
    use nym_credentials_interface::{
        blind_sign, hash_to_scalar, prove_bandwidth_credential, Attribute, Base58, Parameters,
        Signature, VerificationKey,
    };

    #[test]
    fn old_v1_coconut_credential_roundtrip() {
        let voucher_value = 1000000u64;
        let voucher_info = String::from("BandwidthVoucher");
        let serial_number =
            Attribute::try_from_bs58("7Rp3imcuNX3w9se9wm5th8gSvc2czsnMrGsdt5HsrycA").unwrap();
        let binding_number =
            Attribute::try_from_bs58("Auf8yVEgyEAWNHaXUZmimS4n9g5YiYnNYqp6F9BtBe9E").unwrap();
        let signature = Signature::try_from_bs58(
            "ta3pM9ffj5T6YGbwjSBp2W118rcwyP9PXStc\
        7ssb91g5GQYMQHhuTNajbdZcjxUFBFL5rhED8EHpRzE8r432ss3qbPBfpNev4CdkfMkQ3wepyM7hy7q1W6Rn9WmFoZL\
        ZR9j",
        )
        .unwrap();
        let params = Parameters::new(4).unwrap();
        let verification_key = VerificationKey::try_from_bs58("8CFtVVXdwLy4WHMQPE4\
        woe89q3DRHoNxBSchftrEjSBPWA4r4xZv4Y9qSvS5x5bMmFtp7BX6ikECAnuXr5EjXWSsgjirZJmpS5XDUynVfht1cD\
        FWGDvy2XFrRCuoCMotNXi3PoF6wYqdTR9Rqcfoj3i2H5Nid422WBaLtVoC9QNobvpvaqq6vX5PbsSyPayvU8HCXFxM6\
        JjScYpbRTxQtdwefWLrk3LmXyJQBWi7c2VAhSxu9msp7VTBycqdwQNgxHETStZuwXsozxaGQ2KssVUCaaoYPR4g2RqK\
        UAvtWwA7pMiAQNcbkXcbsjCgVjWaCpMWC37XA31cLcFf3zbjHD9e5tXjAcqa4M89fbFhuvvSXxowSAZ5NoWrN32kd5d\
        wxJm1JW3Tt2h6yDDBe84oMy71462dZn7N78DVk2mFNGwBCibrZWA7oUzRBMfYxiQrksoFcou7QfLLd58zoNYmPQPt84\
        1VpQopEBfdQ7Nf9zoXxBt3zMy7g5NsFGvzh7KTbDUyeeXrdkKJPQBs6dqaizr9sS8CPPmR4uk96vDTRh8CJ5FbSsmb8\
        nP71dRvvwRZJHGzwYirMo6SXS3ZYxFuiA3mkxYuqDHCwkTWDuRCcAaztrDYRZg7VCMo4Q446AaEso5eqpeWpHZQt53E\
        ZRpqmNYKASGwMhTeEHPSLgSmtoAAUcaRWpGRzYfd6kzEma8tdGLwyP4rLXgvSvtDLP37dU7YgF3LEXbGAz57U9ATy46\
        6sroLpHPdaCWB8RF11wvB6Tu196JnJd2KyQBP1iUWP3rtZs3GhAF1QVcxquh8BqDZzAcpQ6wCS1P9c5GxKgww77FVF5\
        Kp83XtoxSrw3GaYVyKTGxNh3vcKPR31txCjTxPaN2fg7TaPLhoQJX4YaAroFSXqrqbbRsisuHhhCeUP2YwDjHedes9y")
            .unwrap();
        let theta = prove_bandwidth_credential(
            &params,
            &verification_key,
            &signature,
            &serial_number,
            &binding_number,
        )
        .unwrap();

        let credential = OldV1Credential {
            n_params: 4,
            theta,
            voucher_value,
            voucher_info,
            epoch_id: 42,
        };

        let serialized_credential = credential.as_bytes();
        let deserialized_credential = OldV1Credential::from_bytes(&serialized_credential).unwrap();

        assert_eq!(credential, deserialized_credential);
    }

    #[test]
    fn credential_roundtrip() {
        // make valid request
        let params = bandwidth_credential_params();
        let keypair = nym_credentials_interface::keygen(params);

        let issuance = IssuanceBandwidthCredential::new_freepass(None);
        let sig_req = issuance.prepare_for_signing();
        let pub_attrs_hashed = sig_req
            .public_attributes_plain
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();
        let pub_attrs = pub_attrs_hashed.iter().collect::<Vec<_>>();
        let blind_sig = blind_sign(
            params,
            keypair.secret_key(),
            &sig_req.blind_sign_request,
            &pub_attrs,
        )
        .unwrap();
        let sig = blind_sig
            .unblind(
                keypair.verification_key(),
                &sig_req.pedersen_commitments_openings,
            )
            .unwrap();

        let issued = issuance.into_issued_credential(sig, 42);
        let spending = issued
            .prepare_for_spending(keypair.verification_key())
            .unwrap();

        let with_epoch = CredentialSpendingRequest { data: spending };

        let bytes = with_epoch.to_bytes();
        let recovered = CredentialSpendingRequest::try_from_bytes(&bytes).unwrap();

        assert_eq!(with_epoch, recovered);
    }
}
