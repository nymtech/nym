// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use nym_crypto::asymmetric::ed25519;
use nym_validator_client::{DirectSecp256k1HdWallet, signing::signer::OfflineSigner};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::types::VpnApiTime;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct JwtHeader {
    /// Type that is always "jwt"
    typ: String,
    /// Either "ES256K" or "ECDSA"
    ///
    /// NOTE: These JWTs are not meant to be used outside NymVPN API, so the encoding of their signatures
    ///       serialisation formats do not follow any RFCs, because NymVPN software creates and consumes them
    ///
    /// Elliptic curve signatures using secp256k1 scheme, sadly not in the table of standard algorithms in
    /// https://www.rfc-editor.org/rfc/rfc7518#section-3.1. This scheme is chosen to match the signatures used
    /// in the Nyx chain based on the Cosmos SDK.
    ///
    alg: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct JwtPayload {
    /// Issued at, as a Unix epoch, timezone is UTC
    iat: u128,
    /// The number of seconds the token is valid for, after the issued at UTC epoch
    exp: u8,
    /// The base58 public key of the account (for signature verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pubkey: Option<String>,
    /// The subject is the Cosmos account id of the user, or the public key of the device
    sub: String,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) struct Jwt {
    header: JwtHeader,
    payload: JwtPayload,
    signature: String,
    jwt: String,
}

impl Jwt {
    pub fn new_secp256k1(wallet: &DirectSecp256k1HdWallet) -> Jwt {
        let timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs() as u128;
        tracing::debug!("timestamp: {}", timestamp);
        Jwt::new_secp256k1_with_now(wallet, timestamp)
    }

    pub fn new_secp256k1_synced(wallet: &DirectSecp256k1HdWallet, remote_time: VpnApiTime) -> Jwt {
        Jwt::new_secp256k1_with_now(wallet, remote_time.estimate_remote_now_unix())
    }

    pub fn new_secp256k1_with_now(wallet: &DirectSecp256k1HdWallet, now: u128) -> Jwt {
        let account = wallet.get_accounts().unwrap(); // TODO: result
        let address = account[0].address();
        let public_key = account[0].public_key().to_bytes();

        let header = JwtHeader {
            typ: "JWT".to_string(),
            alg: "ES256K".to_string(),
        };
        let payload = JwtPayload {
            iat: now,
            exp: 30,
            pubkey: Some(bs58::encode(&public_key).into_string()),
            sub: address.to_string(),
        };

        let header_base64 = base64_url::encode(&json!(header.clone()).to_string());
        let payload_base64 = base64_url::encode(&json!(payload.clone()).to_string());
        let message = format!("{header_base64}.{payload_base64}").into_bytes();

        let signature = wallet.sign_raw(address, message).unwrap(); // TODO: result
        let signature_bytes = signature.to_bytes().to_vec();

        let signature_base64 = base64_url::encode(&signature_bytes);

        let jwt = format!("{header_base64}.{payload_base64}.{signature_base64}");

        Jwt {
            header,
            payload,
            signature: signature_base64,
            jwt,
        }
    }

    pub fn new_ecdsa(key_pair: &ed25519::KeyPair) -> Jwt {
        let timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs() as u128;
        Jwt::new_ecdsa_with_now(key_pair, timestamp)
    }

    pub fn new_ecdsa_synced(key_pair: &ed25519::KeyPair, remote_time: VpnApiTime) -> Jwt {
        Jwt::new_ecdsa_with_now(key_pair, remote_time.estimate_remote_now_unix())
    }

    pub fn new_ecdsa_with_now(key_pair: &ed25519::KeyPair, now: u128) -> Jwt {
        let header = JwtHeader {
            typ: "JWT".to_string(),
            alg: "ECDSA".to_string(),
        };
        let payload = JwtPayload {
            iat: now,
            exp: 30,
            pubkey: None,
            sub: key_pair.public_key().to_base58_string(),
        };

        let header_base64 = base64_url::encode(&json!(header.clone()).to_string());
        let payload_base64 = base64_url::encode(&json!(payload.clone()).to_string());
        let message = format!("{header_base64}.{payload_base64}").into_bytes();
        let to_sign = Sha256::digest(&message);

        let signature = key_pair.private_key().sign(to_sign);
        let signature_bytes = signature.to_bytes();

        let signature_base64 = base64_url::encode(&signature_bytes);

        let jwt = format!("{header_base64}.{payload_base64}.{signature_base64}");

        Jwt {
            header,
            payload,
            signature: signature_base64,
            jwt,
        }
    }
}

impl fmt::Display for Jwt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.jwt)
    }
}

#[cfg(test)]
mod tests {
    use nym_crypto::asymmetric::ed25519;
    use nym_validator_client::DirectSecp256k1HdWallet;

    use super::*;

    fn get_secp256k1_keypair() -> DirectSecp256k1HdWallet {
        let mnemonic = "kiwi ketchup mix canvas curve ribbon congress method feel frozen act annual aunt comfort side joy mesh palace tennis cannon orange name tortoise piece";
        let mnemonic = bip39::Mnemonic::parse(mnemonic).unwrap();
        DirectSecp256k1HdWallet::from_mnemonic("n", mnemonic)
    }

    #[test]
    fn secp256k1_jwt_matches_javascript() {
        let now = 1722535718u128;
        let jwt_expected_from_js_snapshot = "eyJhbGciOiJFUzI1NksiLCJ0eXAiOiJKV1QifQ.eyJleHAiOjMwLCJpYXQiOjE3MjI1MzU3MTgsInB1YmtleSI6IndneFpTM25CbWJBd2Nud0FpTnlDTjE5dWNTZHo5cVdkUXlidDJyYWtQVUhyIiwic3ViIjoibjE4Y2phemx4dTd0ODZzODV3YWwzbTdobms4ZXE3cGNmYWtoZHE1MyJ9.qxwY96D-vzMxHWZ840_l6YVDeuZeEkYqz2FaPS8ROztEqipXWCYUi8M1YTH1ZUuNyjgDAMS3NAM3hYvY09ODWQ";
        let jwt_expected_from_js_snapshot_components: Vec<&str> =
            jwt_expected_from_js_snapshot.split('.').collect();

        let wallet = get_secp256k1_keypair();
        let jwt = Jwt::new_secp256k1_with_now(&wallet, now);

        let jwt_str = jwt.to_string();
        let jwt_components: Vec<&str> = jwt_str.split('.').collect();

        if jwt_str != jwt_expected_from_js_snapshot {
            println!("== secp256k1 / ED256K1 ==");
            println!(
                "jwt_expected_from_js_snapshot = {}",
                jwt_expected_from_js_snapshot
            );
            println!("jwt_str                       = {}", jwt_str);
        }

        assert_eq!(
            jwt_expected_from_js_snapshot_components[0],
            jwt_components[0]
        ); // header
        assert_eq!(
            jwt_expected_from_js_snapshot_components[1],
            jwt_components[1]
        ); // payload
        assert_eq!(
            jwt_expected_from_js_snapshot_components[2],
            jwt_components[2]
        ); // signature

        assert_eq!(jwt_expected_from_js_snapshot, jwt_str); // whole JWT strings
    }

    fn get_ed25519_keypair() -> ed25519::KeyPair {
        // let mnemonic = "kiwi ketchup mix canvas curve ribbon congress method feel frozen act annual aunt comfort side joy mesh palace tennis cannon orange name tortoise piece";
        let private_key_base58 = "9JqXnPvTrWkq1Yq66d8GbXrcz5eryAhPZvZ46cEsBPUY";
        let public_key_base58 = "4SPdxfBYsuARBw6REQQa5vFiKcvmYiet9sSWqb751i3Z";

        let private_key = bs58::decode(private_key_base58).into_vec().unwrap();
        let public_key = bs58::decode(public_key_base58).into_vec().unwrap();

        ed25519::KeyPair::from_bytes(&private_key, &public_key).unwrap()
    }

    #[test]
    fn ed25519_ecdsa_jwt_matches_javascript() {
        let now = 1722535718u128;
        let jwt_expected_from_js_snapshot = "eyJhbGciOiJFQ0RTQSIsInR5cCI6IkpXVCJ9.eyJleHAiOjMwLCJpYXQiOjE3MjI1MzU3MTgsInN1YiI6IjRTUGR4ZkJZc3VBUkJ3NlJFUVFhNXZGaUtjdm1ZaWV0OXNTV3FiNzUxaTNaIn0.wSd8y1QdqOVYLf2uTMlnymmiIPQwpxXWd2QvPZ-XqV8O1PNiurQO5JPU65SnaOfggJVA5pnAgZLbj9ciOJKIDg";
        let jwt_expected_from_js_snapshot_components: Vec<&str> =
            jwt_expected_from_js_snapshot.split('.').collect();

        let key_pair = get_ed25519_keypair();

        let jwt = Jwt::new_ecdsa_with_now(&key_pair, now);

        let jwt_str = jwt.to_string();
        let jwt_components: Vec<&str> = jwt_str.split('.').collect();

        if jwt_str != jwt_expected_from_js_snapshot {
            println!("== ed25519 / ECDSA ==");
            println!(
                "jwt_expected_from_js_snapshot = {}",
                jwt_expected_from_js_snapshot
            );
            println!("jwt_str                       = {}", jwt_str);
        }

        assert_eq!(
            jwt_expected_from_js_snapshot_components[0],
            jwt_components[0]
        ); // header
        assert_eq!(
            jwt_expected_from_js_snapshot_components[1],
            jwt_components[1]
        ); // payload
        assert_eq!(
            jwt_expected_from_js_snapshot_components[2],
            jwt_components[2]
        ); // signature

        assert_eq!(jwt_expected_from_js_snapshot, jwt_str); // whole JWT strings
    }
}
