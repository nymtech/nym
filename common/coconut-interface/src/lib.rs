// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

use error::CoconutInterfaceError;

pub use nymcoconut::*;

#[derive(Debug, Serialize, Deserialize, Getters, CopyGetters, Clone, PartialEq)]
pub struct Credential {
    #[getset(get = "pub")]
    n_params: u32,
    #[getset(get = "pub")]
    theta: Theta,
    voucher_value: u64,
    voucher_info: String,
}
impl Credential {
    pub fn new(
        n_params: u32,
        theta: Theta,
        voucher_value: u64,
        voucher_info: String,
    ) -> Credential {
        Credential {
            n_params,
            theta,
            voucher_value,
            voucher_info,
        }
    }

    pub fn voucher_value(&self) -> u64 {
        self.voucher_value
    }

    pub fn verify(&self, verification_key: &VerificationKey) -> bool {
        let params = Parameters::new(self.n_params).unwrap();
        let public_attributes = vec![
            self.voucher_value.to_string().as_bytes(),
            self.voucher_info.as_bytes(),
        ]
        .iter()
        .map(hash_to_scalar)
        .collect::<Vec<Attribute>>();
        nymcoconut::verify_credential(&params, verification_key, &self.theta, &public_attributes)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let n_params_bytes = self.n_params.to_be_bytes();
        let theta_bytes = self.theta.to_bytes();
        let theta_bytes_len = theta_bytes.len();
        let voucher_value_bytes = self.voucher_value.to_be_bytes();
        let voucher_info_bytes = self.voucher_info.as_bytes();
        let voucher_info_len = voucher_info_bytes.len();

        let mut bytes = Vec::with_capacity(28 + theta_bytes_len + voucher_info_len);
        bytes.extend_from_slice(&n_params_bytes);
        bytes.extend_from_slice(&(theta_bytes_len as u64).to_be_bytes());
        bytes.extend_from_slice(&theta_bytes);
        bytes.extend_from_slice(&voucher_value_bytes);
        bytes.extend_from_slice(&voucher_info_bytes);

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
        let voucher_info = String::from_utf8(bytes[20 + theta_len as usize..].to_vec())
            .map_err(|e| CoconutError::Deserialization(e.to_string()))?;

        Ok(Credential {
            n_params,
            theta,
            voucher_value,
            voucher_info,
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

#[derive(Serialize, Deserialize, Getters, CopyGetters)]
pub struct VerifyCredentialBody {
    #[getset(get = "pub")]
    credential: Credential,
}

impl VerifyCredentialBody {
    pub fn new(credential: Credential) -> VerifyCredentialBody {
        VerifyCredentialBody { credential }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyCredentialResponse {
    pub verification_result: bool,
}

impl VerifyCredentialResponse {
    pub fn new(verification_result: bool) -> Self {
        VerifyCredentialResponse {
            verification_result,
        }
    }
}

//  All strings are base58 encoded representations of structs
#[derive(Clone, Serialize, Deserialize, Debug, Getters, CopyGetters)]
pub struct BlindSignRequestBody {
    #[getset(get = "pub")]
    blind_sign_request: BlindSignRequest,
    #[getset(get = "pub")]
    tx_hash: String,
    #[getset(get = "pub")]
    signature: String,
    public_attributes: Vec<String>,
    #[getset(get = "pub")]
    public_attributes_plain: Vec<String>,
    #[getset(get = "pub")]
    total_params: u32,
}

impl BlindSignRequestBody {
    pub fn new(
        blind_sign_request: &BlindSignRequest,
        tx_hash: String,
        signature: String,
        public_attributes: &[Attribute],
        public_attributes_plain: Vec<String>,
        total_params: u32,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            blind_sign_request: blind_sign_request.clone(),
            tx_hash,
            signature,
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
            public_attributes_plain,
            total_params,
        }
    }

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlindedSignatureResponse {
    pub remote_key: [u8; 32],
    pub encrypted_signature: Vec<u8>,
}

impl BlindedSignatureResponse {
    pub fn new(encrypted_signature: Vec<u8>, remote_key: [u8; 32]) -> BlindedSignatureResponse {
        BlindedSignatureResponse {
            encrypted_signature,
            remote_key,
        }
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, CoconutInterfaceError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.remote_key.to_vec();
        bytes.extend_from_slice(&self.encrypted_signature);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoconutInterfaceError> {
        if bytes.len() < 32 {
            return Err(CoconutInterfaceError::InvalidByteLength(bytes.len(), 32));
        }
        let mut remote_key = [0u8; 32];
        remote_key.copy_from_slice(&bytes[..32]);
        let encrypted_signature = bytes[32..].to_vec();
        Ok(BlindedSignatureResponse {
            remote_key,
            encrypted_signature,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct VerificationKeyResponse {
    pub key: VerificationKey,
}

impl VerificationKeyResponse {
    pub fn new(key: VerificationKey) -> VerificationKeyResponse {
        VerificationKeyResponse { key }
    }
}

#[cfg(test)]
mod tests {
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
            serial_number,
            binding_number,
        )
        .unwrap();
        let credential = Credential::new(4, theta, voucher_value, voucher_info);

        let serialized_credential = credential.as_bytes();
        let deserialized_credential = Credential::from_bytes(&serialized_credential).unwrap();

        assert_eq!(credential, deserialized_credential);
    }
}
