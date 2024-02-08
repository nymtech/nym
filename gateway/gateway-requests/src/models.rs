// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::GatewayRequestsError;
use nym_credentials::coconut::bandwidth::CredentialSpendingData;
use nym_credentials_interface::{CoconutError, VerifyCredentialRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialSpendingWithEpoch {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier
    /// could use correct verification key for validation.
    pub epoch_id: u64,
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

impl CredentialSpendingWithEpoch {
    pub fn new(data: CredentialSpendingData, epoch_id: u64) -> Self {
        CredentialSpendingWithEpoch { data, epoch_id }
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
        bytes.extend_from_slice(&self.epoch_id.to_be_bytes());

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

        Ok(CredentialSpendingWithEpoch {
            data: CredentialSpendingData {
                embedded_private_attributes,
                verify_credential_request: theta,
                public_attributes_plain,
                typ,
            },
            epoch_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_credentials::coconut::bandwidth::bandwidth_credential_params;
    use nym_credentials::IssuanceBandwidthCredential;
    use nym_credentials_interface::{blind_sign, hash_to_scalar};

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

        let issued = issuance.into_issued_credential(sig);
        let spending = issued
            .prepare_for_spending(keypair.verification_key())
            .unwrap();

        let with_epoch = CredentialSpendingWithEpoch {
            data: spending,
            epoch_id: 42,
        };

        let bytes = with_epoch.to_bytes();
        let recovered = CredentialSpendingWithEpoch::try_from_bytes(&bytes).unwrap();

        assert_eq!(with_epoch, recovered);
    }
}
