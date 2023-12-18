// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// for time being assume the bandwidth credential consists of public identity of the requester
// and private (though known... just go along with it) infinite bandwidth value
// right now this has no double-spending protection, spender binding, etc
// it's the simplest possible case

use cosmrs::tendermint::hash::Algorithm;
use cosmrs::tendermint::Hash;
use nym_compact_ecash::{
    scheme::{
        keygen::KeyPairUser,
        withdrawal::{RequestInfo, WithdrawalRequest},
    },
    setup::GroupParameters,
    withdrawal_request,
};
use nym_crypto::asymmetric::{encryption, identity};

use crate::error::Error;

pub const PUBLIC_ATTRIBUTES: u32 = 2;
pub const PRIVATE_ATTRIBUTES: u32 = 2;
pub const TOTAL_ATTRIBUTES: u32 = PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES;

pub struct BandwidthVoucher {
    // the plain text value (e.g., bandwidth) encoded in this voucher
    voucher_value_plain: String,
    // the plain text information
    voucher_info_plain: String,
    // the hash of the deposit transaction
    tx_hash: Hash,
    // base58 encoded private key ensuring the depositer requested these attributes
    signing_key: identity::PrivateKey,
    // base58 encoded private key ensuring only this client receives the signature share
    encryption_key: encryption::PrivateKey,
    ecash_keypair: KeyPairUser,
    withdrawal_request_info: RequestInfo,
    withdrawal_request: WithdrawalRequest,
}

impl BandwidthVoucher {
    pub fn new(
        params: &GroupParameters,
        voucher_value: String,
        voucher_info: String,
        tx_hash: Hash,
        signing_key: identity::PrivateKey,
        encryption_key: encryption::PrivateKey,
        ecash_keypair: KeyPairUser,
    ) -> Self {
        let voucher_value_plain = voucher_value.clone();
        let voucher_info_plain = voucher_info.clone();
        let (withdrawal_request, withdrawal_request_info) =
            withdrawal_request(params, &ecash_keypair.secret_key()).unwrap();
        BandwidthVoucher {
            voucher_value_plain,
            voucher_info_plain,
            tx_hash,
            signing_key,
            encryption_key,
            ecash_keypair,
            withdrawal_request_info,
            withdrawal_request,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let voucher_value_plain_b = self.voucher_value_plain.as_bytes();
        let voucher_info_plain_b = self.voucher_info_plain.as_bytes();
        let tx_hash_b = self.tx_hash.as_bytes();
        let signing_key_b = self.signing_key.to_bytes();
        let encryption_key_b = self.encryption_key.to_bytes();
        let ecash_key_b = self.ecash_keypair.to_bytes();
        let withdrawal_request_b = self.withdrawal_request.to_bytes();
        let withdrawal_request_info_b = self.withdrawal_request_info.to_bytes();

        let mut ret = Vec::new();

        ret.extend_from_slice(tx_hash_b);
        ret.extend_from_slice(&signing_key_b);
        ret.extend_from_slice(&encryption_key_b);
        ret.extend_from_slice(&ecash_key_b);
        ret.extend_from_slice(&(voucher_value_plain_b.len() as u64).to_be_bytes());
        ret.extend_from_slice(&(voucher_info_plain_b.len() as u64).to_be_bytes());
        ret.extend_from_slice(&(withdrawal_request_b.len() as u64).to_be_bytes());
        ret.extend_from_slice(&(withdrawal_request_info_b.len() as u64).to_be_bytes());
        ret.extend_from_slice(voucher_value_plain_b);
        ret.extend_from_slice(voucher_info_plain_b);
        ret.extend_from_slice(&withdrawal_request_b);
        ret.extend_from_slice(&withdrawal_request_info_b);

        ret
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 32 * 3 + 80 + 4 * 8 {
            return Err(Error::BandwidthVoucherDeserializationError(format!(
                "Less then {} bytes needed",
                32 * 3 + 80 + 4 * 8
            )));
        }
        let mut buff = [0u8; 32];
        let mut small_buff = [0u8; 8];

        buff.copy_from_slice(&bytes[0..32]);
        let tx_hash = Hash::from_bytes(Algorithm::Sha256, &buff).map_err(|_| {
            Error::BandwidthVoucherDeserializationError(String::from("Invalid transaction Hash"))
        })?;

        buff.copy_from_slice(&bytes[32..2 * 32]);
        let signing_key = identity::PrivateKey::from_bytes(&buff).map_err(|_| {
            Error::BandwidthVoucherDeserializationError(String::from("Invalid key"))
        })?;

        buff.copy_from_slice(&bytes[2 * 32..3 * 32]);
        let encryption_key = encryption::PrivateKey::from_bytes(&buff).map_err(|_| {
            Error::BandwidthVoucherDeserializationError(String::from("Invalid key"))
        })?;

        //ecash key
        let ecash_keypair = KeyPairUser::from_bytes(&bytes[3 * 32..3 * 32 + 80])?;

        small_buff.copy_from_slice(&bytes[3 * 32 + 80..3 * 32 + 80 + 8]);
        let voucher_value_plain_no = u64::from_be_bytes(small_buff) as usize;
        small_buff.copy_from_slice(&bytes[3 * 32 + 80 + 8..3 * 32 + 80 + 2 * 8]);
        let voucher_info_plain_no = u64::from_be_bytes(small_buff) as usize;
        small_buff.copy_from_slice(&bytes[3 * 32 + 80 + 2 * 8..3 * 32 + 80 + 3 * 8]);
        let withdrawal_request_no = u64::from_be_bytes(small_buff) as usize;
        small_buff.copy_from_slice(&bytes[3 * 32 + 80 + 3 * 8..3 * 32 + 80 + 4 * 8]);
        let withdrawal_request_info_no = u64::from_be_bytes(small_buff) as usize;

        let total_length = 32 * 3
            + 80
            + 4 * 8
            + voucher_value_plain_no
            + voucher_info_plain_no
            + withdrawal_request_no
            + withdrawal_request_info_no;
        if bytes.len() != total_length {
            return Err(Error::BandwidthVoucherDeserializationError(format!(
                "Expected {total_length} bytes",
            )));
        }

        let utf_err = |_| {
            Err(Error::BandwidthVoucherDeserializationError(String::from(
                "Invalid UTF8 string",
            )))
        };
        let mut var_length_pointer = 32 * 3 + 80 + 4 * 8;
        let voucher_value_plain = String::from_utf8(
            bytes[var_length_pointer..var_length_pointer + voucher_value_plain_no].to_vec(),
        )
        .or_else(utf_err)?;
        var_length_pointer += voucher_value_plain_no;

        let voucher_info_plain = String::from_utf8(
            bytes[var_length_pointer..var_length_pointer + voucher_info_plain_no].to_vec(),
        )
        .or_else(utf_err)?;
        var_length_pointer += voucher_info_plain_no;

        let withdrawal_request = WithdrawalRequest::try_from(
            &bytes[var_length_pointer..var_length_pointer + withdrawal_request_no],
        )?;
        var_length_pointer += withdrawal_request_no;

        let withdrawal_request_info = RequestInfo::try_from(
            &bytes[var_length_pointer..var_length_pointer + withdrawal_request_info_no],
        )?;

        Ok(Self {
            voucher_value_plain,
            voucher_info_plain,
            tx_hash,
            signing_key,
            encryption_key,
            ecash_keypair,
            withdrawal_request,
            withdrawal_request_info,
        })
    }

    pub fn tx_hash(&self) -> &Hash {
        &self.tx_hash
    }

    pub fn encryption_key(&self) -> &encryption::PrivateKey {
        &self.encryption_key
    }

    pub fn ecash_keypair(&self) -> &KeyPairUser {
        &self.ecash_keypair
    }

    pub fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.withdrawal_request
    }

    pub fn withdrawal_request_info(&self) -> &RequestInfo {
        &self.withdrawal_request_info
    }

    pub fn get_voucher_value(&self) -> String {
        self.voucher_value_plain.clone()
    }

    pub fn get_public_attributes_plain(&self) -> Vec<String> {
        vec![
            self.voucher_value_plain.clone(),
            self.voucher_info_plain.clone(),
        ]
    }

    pub fn sign(&self, request: &WithdrawalRequest) -> identity::Signature {
        let mut message = request.to_bytes();
        message.extend_from_slice(self.tx_hash.to_string().as_bytes());
        self.signing_key.sign(&message)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmrs::tendermint::hash::Algorithm;
    use nym_compact_ecash::{generate_keypair_user, Base58};
    use rand::rngs::OsRng;

    fn voucher_fixture() -> BandwidthVoucher {
        let params = GroupParameters::new().unwrap();
        let mut rng = OsRng;
        BandwidthVoucher::new(
            &params,
            "1234".to_string(),
            "voucher info".to_string(),
            Hash::from_bytes(Algorithm::Sha256, &[0; 32]).unwrap(),
            identity::PrivateKey::from_base58_string(
                identity::KeyPair::new(&mut rng)
                    .private_key()
                    .to_base58_string(),
            )
            .unwrap(),
            encryption::PrivateKey::from_bytes(
                &encryption::KeyPair::new(&mut rng).private_key().to_bytes(),
            )
            .unwrap(),
            generate_keypair_user(&params),
        )
    }

    #[test]
    fn serde_voucher() {
        let voucher = voucher_fixture();
        let bytes = voucher.to_bytes();
        let deserialized_voucher = BandwidthVoucher::try_from_bytes(&bytes).unwrap();
        assert_eq!(
            voucher.voucher_value_plain,
            deserialized_voucher.voucher_value_plain
        );
        assert_eq!(
            voucher.voucher_info_plain,
            deserialized_voucher.voucher_info_plain
        );
        assert_eq!(voucher.tx_hash, deserialized_voucher.tx_hash);
        assert_eq!(
            voucher.signing_key.to_string(),
            deserialized_voucher.signing_key.to_string()
        );
        assert_eq!(
            voucher.encryption_key.to_string(),
            deserialized_voucher.encryption_key.to_string()
        );
        assert_eq!(
            voucher.withdrawal_request_info.to_bytes(),
            deserialized_voucher.withdrawal_request_info.to_bytes()
        );
        assert_eq!(
            voucher.withdrawal_request.to_bs58(),
            deserialized_voucher.withdrawal_request.to_bs58()
        );
    }
}
