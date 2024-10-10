// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiLibError;
use nym_compact_ecash::scheme::keygen::KeyPairUser;
use nym_compact_ecash::scheme::withdrawal::RequestInfo;
use nym_compact_ecash::{
    aggregate_wallets, issue_verify, withdrawal_request, Base58, BlindedSignature,
    VerificationKeyAuth, WithdrawalRequest,
};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    IssuedTicketBook,
};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::{ecash_default_expiration_date, EcashTime};
use nym_vpn_api_requests::api::v1::ticketbook::models::{
    MasterVerificationKeyResponse, PartialVerificationKeysResponse, TicketbookRequest,
    TicketbookWalletSharesResponse, WalletShare,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::Date;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use wasm_utils::console_error;
use zeroize::Zeroizing;

pub mod error;

#[derive(Tsify, Debug, Default, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WalletShares(Vec<WalletShare>);

pub type WalletIssuers = PartialVerificationKeysResponse;

impl From<Vec<WalletShare>> for WalletShares {
    fn from(shares: Vec<WalletShare>) -> Self {
        WalletShares(shares)
    }
}

#[derive(Tsify, Debug, Default, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct NymIssuanceTicketbookOpts {
    #[tsify(optional)]
    pub ticketbook_type: Option<String>,

    // bs58-encoded user secret key used for seeding the ecash crypto keypair generation
    // I reiterate, this is a **SECRET** key, not a public key.
    #[tsify(optional)]
    pub user_secret_key: Option<String>,
}

#[wasm_bindgen]
#[derive(Debug)]
#[allow(dead_code)]
pub struct NymIssuanceTicketbook {
    /// ecash keypair related to the credential
    ecash_keypair: KeyPairUser,

    withdrawal_request: WithdrawalRequest,

    ticketbook_type: TicketType,

    expiration_date: Date,

    request_info: Zeroizing<RequestInfo>,
}

#[wasm_bindgen]
impl NymIssuanceTicketbook {
    #[wasm_bindgen(constructor)]
    pub fn new(opts: NymIssuanceTicketbookOpts) -> Result<NymIssuanceTicketbook, VpnApiLibError> {
        let ecash_keypair = match opts.user_secret_key {
            None => KeyPairUser::new(),
            Some(maybe_sk) => {
                let pk = ed25519::PrivateKey::from_base58_string(maybe_sk)
                    .map(Zeroizing::new)
                    .map_err(|_| VpnApiLibError::MalformedEd25519Key)?;
                let bytes = Zeroizing::new(pk.to_bytes());
                KeyPairUser::new_seeded(&bytes)
            }
        };

        let ticketbook_type = match opts.ticketbook_type {
            None => TicketType::default(),
            Some(typ) => typ
                .parse()
                .map_err(|_| VpnApiLibError::MalformedTicketType)?,
        };

        let expiration_date = ecash_default_expiration_date();

        let (withdrawal_request, request_info) = withdrawal_request(
            ecash_keypair.secret_key(),
            expiration_date.ecash_unix_timestamp(),
            ticketbook_type.encode(),
        )?;

        Ok(NymIssuanceTicketbook {
            ecash_keypair,
            withdrawal_request,
            ticketbook_type: Default::default(),
            expiration_date,
            request_info: Zeroizing::new(request_info),
        })
    }

    #[wasm_bindgen(js_name = "buildRequestPayload")]
    pub fn build_request_payload(&self, is_freepass_request: bool) -> String {
        serde_json::to_string(&TicketbookRequest {
            withdrawal_request: self.withdrawal_request.clone().into(),
            ecash_pubkey: self.ecash_keypair.public_key(),
            expiration_date: self.expiration_date,
            ticketbook_type: self.ticketbook_type,
            is_freepass_request,
        })
        .unwrap()
    }

    #[wasm_bindgen(js_name = "getWithdrawalRequest")]
    pub fn get_encoded_withdrawal_request(&self) -> String {
        self.withdrawal_request.to_bs58()
    }

    #[wasm_bindgen(js_name = "getEncodedPublicKey")]
    pub fn get_encoded_public_key(&self) -> String {
        self.ecash_keypair.public_key().to_bs58()
    }

    //
    // #[wasm_bindgen(js_name = "unblindShare")]
    // pub fn unblind_share(&self, share: UnblindableShare) -> Result<CredentialWrapper, ZkNymError> {
    //     let blinded_sig = BlindedSignature::try_from_bs58(share.blinded_share_bs58)?;
    //     let vk = VerificationKey::try_from_bs58(share.issuer_key_bs58)?;
    //
    //     Ok(blinded_sig
    //         .unblind(&vk, &self.pedersen_commitments_openings)
    //         .into())
    // }
    //
    #[wasm_bindgen(js_name = "unblindWalletShares")]
    pub fn unblind_wallet_shares(
        self,
        shares: JsValue,
        issuers: WalletIssuers,
        master_key: MasterVerificationKeyResponse,
    ) -> Result<NymIssuedTicketbook, VpnApiLibError> {
        // we couldn't derive all the required abi traits due to crypto types deep in the stack
        let shares: TicketbookWalletSharesResponse = serde_wasm_bindgen::from_value(shares)?;

        if shares.epoch_id != issuers.epoch_id {
            console_error!(
                "the provided shares and issuers are not from the same epoch! {} and {}",
                shares.epoch_id,
                issuers.epoch_id
            );
            return Err(VpnApiLibError::InconsistentEpochId {
                shares: shares.epoch_id,
                issuers: issuers.epoch_id,
            });
        }

        let master_vk = VerificationKeyAuth::try_from_bs58(master_key.bs58_encoded_key)?;

        let mut decoded_keys = HashMap::new();
        for key in issuers.keys {
            let vk = VerificationKeyAuth::try_from_bs58(key.bs58_encoded_key)?;
            decoded_keys.insert(key.node_index, vk);
        }

        let mut partial_wallets = Vec::new();
        for share in shares.shares {
            let blinded_sig = BlindedSignature::try_from_bs58(share.bs58_encoded_share)?;
            let Some(vk) = decoded_keys.get(&share.node_index) else {
                console_error!("received a share from issuer {} but did not receive a corresponding verification key!", share.node_index);
                continue;
            };

            match issue_verify(
                vk,
                self.ecash_keypair.secret_key(),
                &blinded_sig,
                &self.request_info,
                share.node_index,
            ) {
                Ok(partial_wallet) => partial_wallets.push(partial_wallet),
                Err(err) => {
                    console_error!(
                        "failed to unblind partial wallet corresponding to index {}: {err}",
                        share.node_index
                    )
                }
            }
        }

        let aggregated_wallet = aggregate_wallets(
            &master_vk,
            self.ecash_keypair.secret_key(),
            &partial_wallets,
            &self.request_info,
        )?;

        Ok(NymIssuedTicketbook {
            inner_ticketbook: IssuedTicketBook::new(
                aggregated_wallet.into_wallet_signatures(),
                shares.epoch_id,
                self.ecash_keypair.into(),
                self.ticketbook_type,
                self.expiration_date,
            ),
            master_vk: EpochVerificationKey {
                epoch_id: shares.epoch_id,
                key: master_vk,
            },
            expiration_date_signatures: shares
                .aggregated_expiration_date_signatures
                .map(|s| s.signatures),
            coin_index_signatures: shares
                .aggregated_coin_index_signatures
                .map(|s| s.signatures),
        })
    }
}

#[wasm_bindgen]
pub struct NymIssuedTicketbook {
    inner_ticketbook: IssuedTicketBook,

    master_vk: EpochVerificationKey,
    expiration_date_signatures: Option<AggregatedExpirationDateSignatures>,
    coin_index_signatures: Option<AggregatedCoinIndicesSignatures>,
}

#[wasm_bindgen]
impl NymIssuedTicketbook {
    pub fn serialise(self) -> FullSerialisedNymIssuedTicketbook {
        let serialised = self
            .inner_ticketbook
            .begin_export()
            .with_master_verification_key(&self.master_vk)
            .with_maybe_expiration_date_signatures(&self.expiration_date_signatures)
            .with_maybe_coin_index_signatures(&self.coin_index_signatures)
            .finalize_export();

        FullSerialisedNymIssuedTicketbook {
            serialisation_revision: serialised.revision,
            bs58_encoded_data: bs58::encode(serialised.data).into_string(),
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct FullSerialisedNymIssuedTicketbook {
    pub serialisation_revision: u8,
    pub bs58_encoded_data: String,
}

#[wasm_bindgen(start)]
pub fn main() {
    wasm_utils::console_log!("[rust main]: rust module loaded");
    wasm_utils::console_log!(
        "vpn-api-lib version used:\n{}",
        nym_bin_common::bin_info!().pretty_print()
    );
    wasm_utils::console_log!("[rust main]: setting panic hook");
    wasm_utils::set_panic_hook();
}
