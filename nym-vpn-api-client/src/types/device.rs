// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{fmt, sync::Arc};

use nym_crypto::asymmetric::ed25519;
use sha2::Digest as _;

use crate::{jwt::Jwt, request::UpdateDeviceRequestStatus};

use super::VpnApiTime;

#[derive(Clone)]
pub struct Device {
    keypair: Arc<ed25519::KeyPair>,
}

impl Device {
    pub fn identity_key(&self) -> &ed25519::PublicKey {
        self.keypair.public_key()
    }

    pub(crate) fn jwt(&self, remote_time: Option<VpnApiTime>) -> Jwt {
        match remote_time {
            Some(remote_time) => Jwt::new_ecdsa_synced(&self.keypair, remote_time),
            None => Jwt::new_ecdsa(&self.keypair),
        }
    }

    pub fn sign<M: AsRef<[u8]>>(&self, message: M) -> DeviceSignature {
        let digest = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(message);
            hasher.finalize()
        };

        DeviceSignature(self.keypair.private_key().sign(digest))
    }

    pub fn sign_identity_key(&self) -> DeviceSignature {
        self.sign(self.identity_key().to_base58_string())
    }
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Device {{ {} }}", self.identity_key().to_base58_string())
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Device {{ identity_key: {} }}",
            self.identity_key().to_base58_string()
        )
    }
}

impl From<Arc<ed25519::KeyPair>> for Device {
    fn from(keypair: Arc<ed25519::KeyPair>) -> Self {
        Self { keypair }
    }
}

impl From<ed25519::KeyPair> for Device {
    fn from(keypair: ed25519::KeyPair) -> Self {
        Self {
            keypair: Arc::new(keypair),
        }
    }
}

// In tests we create from a mnemonic, in production these are always created directly from
// the keypair
#[cfg(test)]
impl From<bip39::Mnemonic> for Device {
    fn from(mnemonic: bip39::Mnemonic) -> Self {
        let (entropy, _) = mnemonic.to_entropy_array();
        // Entropy is statically >= 32 bytes, so we can safely extract the first 32
        // bytes
        let seed = &entropy[0..32];

        let signing_key = ed25519::PrivateKey::from_bytes(seed).unwrap();
        let verifying_key = signing_key.public_key();

        let privkey = signing_key.to_bytes().to_vec();
        let pubkey = verifying_key.to_bytes().to_vec();

        let keypair = ed25519::KeyPair::from_bytes(&privkey, &pubkey).unwrap();

        Self {
            keypair: Arc::new(keypair),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatus {
    Active,
    Inactive,
    DeleteMe,
}

impl From<DeviceStatus> for UpdateDeviceRequestStatus {
    fn from(status: DeviceStatus) -> Self {
        match status {
            DeviceStatus::Active => UpdateDeviceRequestStatus::Active,
            DeviceStatus::Inactive => UpdateDeviceRequestStatus::Inactive,
            DeviceStatus::DeleteMe => UpdateDeviceRequestStatus::DeleteMe,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSignature(ed25519::Signature);

impl DeviceSignature {
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    pub fn to_base64_url_string(&self) -> String {
        base64_url::encode(&self.to_bytes())
    }

    pub fn to_base64_string(&self) -> String {
        base64_url::unescape(&self.to_base64_url_string()).to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::test_fixtures::{
        TEST_DEFAULT_DEVICE_IDENTITY_KEY, TEST_DEFAULT_DEVICE_MNEMONIC,
    };

    use super::*;

    fn ed25519_keypair_fixture() -> ed25519::KeyPair {
        // The mnemonic used to generate the keypair
        let _mnemonic = "kiwi ketchup mix canvas curve ribbon congress method feel frozen act annual aunt comfort side joy mesh palace tennis cannon orange name tortoise piece";

        // The corresponding keypair generated from the mnemonic
        let private_key_base58 = "9JqXnPvTrWkq1Yq66d8GbXrcz5eryAhPZvZ46cEsBPUY";
        let public_key_base58 = "4SPdxfBYsuARBw6REQQa5vFiKcvmYiet9sSWqb751i3Z";

        let private_key = bs58::decode(private_key_base58).into_vec().unwrap();
        let public_key = bs58::decode(public_key_base58).into_vec().unwrap();

        ed25519::KeyPair::from_bytes(&private_key, &public_key).unwrap()
    }

    #[test]
    fn verify_ed25519_keypair_fixture() {
        let device = Device::from(
            bip39::Mnemonic::parse("kiwi ketchup mix canvas curve ribbon congress method feel frozen act annual aunt comfort side joy mesh palace tennis cannon orange name tortoise piece").unwrap()
        );
        let expected_keypair = ed25519_keypair_fixture();
        assert_eq!(
            device.keypair.private_key().to_base58_string(),
            expected_keypair.private_key().to_base58_string()
        );
        assert_eq!(
            device.keypair.public_key().to_base58_string(),
            expected_keypair.public_key().to_base58_string()
        );
    }

    #[test]
    fn create_device_from_mnemonic_1() {
        let device = Device::from(bip39::Mnemonic::parse(TEST_DEFAULT_DEVICE_MNEMONIC).unwrap());
        assert_eq!(
            device.identity_key().to_base58_string(),
            TEST_DEFAULT_DEVICE_IDENTITY_KEY
        );
    }

    #[test]
    fn create_device_from_mnemonic_2() {
        let device = Device::from(
            bip39::Mnemonic::parse("kiwi ketchup mix canvas curve ribbon congress method feel frozen act annual aunt comfort side joy mesh palace tennis cannon orange name tortoise piece").unwrap()
        );
        assert_eq!(
            device.identity_key().to_base58_string(),
            "4SPdxfBYsuARBw6REQQa5vFiKcvmYiet9sSWqb751i3Z",
        );
        assert_eq!(
            device.keypair.private_key().to_base58_string(),
            "9JqXnPvTrWkq1Yq66d8GbXrcz5eryAhPZvZ46cEsBPUY",
        );
    }

    #[test]
    fn create_device_from_keypair() {
        let device = Device::from(ed25519_keypair_fixture());
        assert_eq!(
            device.keypair.public_key().to_base58_string(),
            "4SPdxfBYsuARBw6REQQa5vFiKcvmYiet9sSWqb751i3Z",
        );
        assert_eq!(
            device.keypair.private_key().to_base58_string(),
            "9JqXnPvTrWkq1Yq66d8GbXrcz5eryAhPZvZ46cEsBPUY",
        );
    }

    #[test]
    fn sign_identity_key() {
        let device = Device::from(bip39::Mnemonic::parse(TEST_DEFAULT_DEVICE_MNEMONIC).unwrap());
        assert_eq!(
            device.identity_key().to_base58_string(),
            TEST_DEFAULT_DEVICE_IDENTITY_KEY
        );

        let signature = device.sign_identity_key().to_base64_string();
        assert_eq!(
            signature,
            "W5Zv1QhG37Al0QQH/9tqOmv1MU9IjfWP1xDq116GGSu/1Z6cnAW0sOyfrIiqdEleUKJB9wC/HjcsifaogymWAw=="
        );
    }
}
