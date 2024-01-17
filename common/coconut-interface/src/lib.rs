// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

use error::CoconutInterfaceError;

// We list these explicity instead of glob export due to shadowing warnings with the pub tests
// module.
pub use nym_coconut::{
    aggregate_signature_shares, aggregate_verification_keys, blind_sign, hash_to_scalar,
    prepare_blind_sign, prove_bandwidth_credential, Attribute, Base58, BlindSignRequest,
    BlindedSignature, Bytable, CoconutError, KeyPair, Parameters, PrivateAttribute,
    PublicAttribute, Signature, SignatureShare, Theta, VerificationKey, SecretKey
};

#[derive(Debug, Serialize, Deserialize, Getters, CopyGetters, Clone, PartialEq, Eq)]
pub struct Credential {
    #[getset(get = "pub")]
    n_params: u32,

    #[getset(get = "pub")]
    theta: Theta,

    voucher_value: u64,

    voucher_info: String,

    #[getset(get = "pub")]
    epoch_id: u64,
}
impl Credential {
    pub fn new(
        n_params: u32,
        theta: Theta,
        voucher_value: u64,
        voucher_info: String,
        epoch_id: u64,
    ) -> Credential {
        Credential {
            n_params,
            theta,
            voucher_value,
            voucher_info,
            epoch_id,
        }
    }

    pub fn blinded_serial_number(&self) -> String {
        self.theta.blinded_serial_number_bs58()
    }

    pub fn has_blinded_serial_number(
        &self,
        blinded_serial_number_bs58: &str,
    ) -> Result<bool, CoconutInterfaceError> {
        Ok(self
            .theta
            .has_blinded_serial_number(blinded_serial_number_bs58)?)
    }

    pub fn voucher_value(&self) -> u64 {
        self.voucher_value
    }

    pub fn verify(&self, verification_key: &VerificationKey) -> bool {
        let params = Parameters::new(self.n_params).unwrap();

        let hashed_value = hash_to_scalar(self.voucher_value.to_string());
        let hashed_info = hash_to_scalar(&self.voucher_info);
        let public_attributes = &[&hashed_value, &hashed_info];

        nym_coconut::verify_credential(&params, verification_key, &self.theta, public_attributes)
    }

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
        let theta = Theta::from_bytes(&bytes[12..12 + theta_len as usize])
            .map_err(|e| CoconutError::Deserialization(e.to_string()))?;
        eight_byte.copy_from_slice(&bytes[12 + theta_len as usize..20 + theta_len as usize]);
        let voucher_value = u64::from_be_bytes(eight_byte);
        eight_byte.copy_from_slice(&bytes[20 + theta_len as usize..28 + theta_len as usize]);
        let epoch_id = u64::from_be_bytes(eight_byte);
        let voucher_info = String::from_utf8(bytes[28 + theta_len as usize..].to_vec())
            .map_err(|e| CoconutError::Deserialization(e.to_string()))?;

        Ok(Credential {
            n_params,
            theta,
            voucher_value,
            voucher_info,
            epoch_id,
        })
    }
}

impl Bytable for Credential {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.as_bytes()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CoconutError> {
        Credential::from_bytes(slice)
    }
}

impl Base58 for Credential {}

#[cfg(test)]
mod tests {
    use nym_coconut::{prove_bandwidth_credential, Signature};

    use super::*;

    #[test]
    fn serde_coconut_credential() {
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
        let credential = Credential::new(4, theta, voucher_value, voucher_info, 42);

        let serialized_credential = credential.as_bytes();
        let deserialized_credential = Credential::from_bytes(&serialized_credential).unwrap();

        assert_eq!(credential, deserialized_credential);
    }
}
