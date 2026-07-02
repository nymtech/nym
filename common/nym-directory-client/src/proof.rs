// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Verification of CosmWasm raw-store membership proofs.
//!
//! An `abci_query("store/wasm/key", .., prove = true)` returns a two-op ICS23 proof:
//! `ops[0]` (`ics23:iavl`) proves the contract key/value up to the wasm-store root, and
//! `ops[1]` (`ics23:simple`) proves that root up to the multistore root (the `app_hash`).
//! We hand-chain the two `ics23::verify_membership` calls against a trusted `app_hash`

use cosmrs::tendermint::merkle::proof::ProofOp;
use ics23::commitment_proof::Proof;
use ics23::{
    CommitmentProof, HostFunctionsManager, calculate_existence_root, iavl_spec, tendermint_spec,
    verify_membership,
};
use prost::Message;

/// The multistore key under which the CosmWasm module's store is committed.
const WASM_STORE_KEY: &[u8] = b"wasm";

#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    #[error("expected exactly 2 proof ops (ics23:iavl, ics23:simple), got {0}")]
    UnexpectedOpCount(usize),

    #[error("failed to decode the ICS23 commitment proof for op {op}: {source}")]
    Decode {
        op: usize,
        source: prost::DecodeError,
    },

    #[error("proof op {0} is not an existence proof")]
    NotExistenceProof(usize),

    #[error("failed to compute the existence root: {0}")]
    RootCalculation(String),

    #[error(
        "IAVL-layer membership verification failed (key/value not committed in the wasm store)"
    )]
    IavlVerificationFailed,

    #[error(
        "store-layer membership verification failed (wasm store not committed to the app_hash)"
    )]
    StoreVerificationFailed,
}

/// Verify a two-layer CosmWasm store membership proof for `key`/`value` against a
/// trusted multistore `app_hash`.
///
/// `ops` is the `ProofOps` returned by `abci_query("store/wasm/key", .., prove = true)`:
/// `ops[0]` is the `ics23:iavl` proof and `ops[1]` the `ics23:simple` proof. `key` is the
/// full raw wasm key (`0x03 || canonical_addr || contract_key`) and `value` the raw stored
/// bytes. `app_hash` MUST be independently trusted (e.g. the app hash committed in
/// `header[H+1]` for a query at height `H`).
pub fn verify_wasm_store_membership(
    ops: &[ProofOp],
    app_hash: &[u8],
    key: &[u8],
    value: &[u8],
) -> Result<(), ProofError> {
    if ops.len() != 2 {
        return Err(ProofError::UnexpectedOpCount(ops.len()));
    }

    let iavl = CommitmentProof::decode(ops[0].data.as_slice())
        .map_err(|source| ProofError::Decode { op: 0, source })?;
    let store = CommitmentProof::decode(ops[1].data.as_slice())
        .map_err(|source| ProofError::Decode { op: 1, source })?;

    // Layer 1 (ics23:iavl): key/value -> wasm-store root.
    let Some(Proof::Exist(iavl_exist)) = &iavl.proof else {
        return Err(ProofError::NotExistenceProof(0));
    };
    let wasm_store_root = calculate_existence_root::<HostFunctionsManager>(iavl_exist)
        .map_err(|e| ProofError::RootCalculation(e.to_string()))?;

    if !verify_membership::<HostFunctionsManager>(&iavl, &iavl_spec(), &wasm_store_root, key, value)
    {
        return Err(ProofError::IavlVerificationFailed);
    }

    // Layer 2 (ics23:simple): "wasm"/wasm-store-root -> app_hash.
    if !verify_membership::<HostFunctionsManager>(
        &store,
        &tendermint_spec(),
        &app_hash.to_vec(),
        WASM_STORE_KEY,
        &wasm_store_root,
    ) {
        return Err(ProofError::StoreVerificationFailed);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmrs::tendermint::AppHash;
    use cosmrs::tendermint::merkle::proof::ProofOps;
    use nym_validator_client::nyxd::{AccountId, Height};
    use nym_validator_client::rpc::types::ProvableAbciQueryResponse;

    // Phase-0 spike check: the hand-chained ICS23 verifier accepts a real two-op proof
    // and rejects tampering.
    #[test]
    fn verifies_a_live_membership_proof_and_rejects_tampering() -> anyhow::Result<()> {
        // test "admin" key of the existing mainnet mixnet contract
        let contract: AccountId = "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr"
            .parse()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let height: Height = 24499896u32.into();

        let proof = ProofOps {
            ops: vec![
                ProofOp {
                    field_type: "ics23:iavl".to_string(),
                    key: vec![
                        3, 244, 7, 33, 76, 223, 43, 88, 38, 216, 46, 11, 149, 84, 35, 90, 59, 177,
                        232, 179, 191, 57, 251, 173, 211, 178, 70, 187, 59, 57, 130, 43, 151, 97,
                        100, 109, 105, 110,
                    ],
                    data: vec![
                        10, 250, 8, 10, 38, 3, 244, 7, 33, 76, 223, 43, 88, 38, 216, 46, 11, 149,
                        84, 35, 90, 59, 177, 232, 179, 191, 57, 251, 173, 211, 178, 70, 187, 59,
                        57, 130, 43, 151, 97, 100, 109, 105, 110, 18, 42, 34, 110, 49, 116, 120,
                        101, 51, 120, 48, 52, 99, 51, 119, 106, 101, 106, 102, 110, 52, 109, 121,
                        52, 101, 102, 51, 110, 52, 116, 52, 100, 102, 104, 102, 97, 106, 99, 119,
                        113, 103, 57, 116, 34, 26, 14, 8, 1, 24, 1, 32, 1, 42, 6, 0, 2, 134, 192,
                        244, 13, 34, 46, 8, 1, 18, 7, 4, 6, 182, 216, 174, 23, 32, 26, 33, 32, 161,
                        137, 152, 169, 240, 5, 95, 248, 85, 248, 206, 240, 113, 68, 244, 23, 23,
                        70, 213, 72, 7, 189, 187, 155, 18, 57, 41, 53, 83, 71, 253, 107, 34, 46, 8,
                        1, 18, 7, 6, 12, 182, 216, 174, 23, 32, 26, 33, 32, 2, 167, 187, 241, 112,
                        176, 190, 118, 32, 7, 99, 19, 110, 234, 159, 94, 61, 234, 221, 101, 90,
                        226, 220, 188, 243, 251, 102, 197, 27, 39, 253, 208, 34, 44, 8, 1, 18, 40,
                        8, 22, 182, 216, 174, 23, 32, 2, 53, 89, 161, 189, 239, 208, 6, 228, 202,
                        2, 224, 46, 82, 217, 28, 73, 158, 216, 124, 122, 251, 25, 94, 163, 165,
                        104, 174, 243, 147, 164, 173, 32, 34, 44, 8, 1, 18, 40, 10, 42, 182, 216,
                        174, 23, 32, 248, 158, 22, 60, 2, 243, 68, 84, 29, 125, 14, 53, 250, 16,
                        79, 233, 34, 109, 232, 97, 62, 53, 235, 188, 200, 121, 97, 91, 244, 55,
                        136, 58, 32, 34, 44, 8, 1, 18, 40, 12, 84, 182, 216, 174, 23, 32, 102, 75,
                        60, 188, 199, 115, 233, 164, 13, 209, 96, 81, 241, 218, 82, 185, 173, 59,
                        132, 15, 133, 29, 69, 153, 85, 34, 30, 255, 89, 224, 59, 109, 32, 34, 45,
                        8, 1, 18, 41, 14, 132, 1, 182, 216, 174, 23, 32, 147, 228, 32, 45, 181,
                        179, 136, 134, 146, 5, 198, 38, 206, 24, 93, 74, 45, 8, 231, 6, 7, 7, 80,
                        117, 121, 6, 130, 189, 195, 202, 60, 15, 32, 34, 45, 8, 1, 18, 41, 16, 132,
                        2, 182, 216, 174, 23, 32, 115, 15, 48, 63, 121, 64, 57, 71, 7, 164, 118,
                        192, 36, 223, 204, 132, 31, 15, 167, 116, 76, 109, 124, 189, 96, 125, 31,
                        203, 190, 202, 99, 172, 32, 34, 45, 8, 1, 18, 41, 18, 188, 3, 182, 216,
                        174, 23, 32, 199, 21, 161, 113, 246, 169, 57, 84, 113, 220, 243, 68, 67,
                        177, 19, 98, 71, 50, 33, 192, 30, 22, 74, 137, 215, 188, 207, 246, 183, 44,
                        144, 52, 32, 34, 45, 8, 1, 18, 41, 20, 236, 6, 182, 216, 174, 23, 32, 125,
                        214, 119, 138, 58, 215, 230, 76, 154, 42, 11, 22, 202, 62, 34, 213, 107,
                        86, 181, 133, 143, 236, 190, 16, 131, 139, 146, 140, 231, 178, 69, 62, 32,
                        34, 47, 8, 1, 18, 8, 22, 190, 11, 234, 218, 174, 23, 32, 26, 33, 32, 58,
                        49, 117, 16, 200, 12, 12, 7, 3, 40, 73, 82, 29, 216, 157, 119, 165, 141,
                        67, 86, 196, 16, 248, 203, 24, 139, 14, 229, 105, 210, 39, 213, 34, 45, 8,
                        1, 18, 41, 26, 136, 29, 234, 218, 174, 23, 32, 69, 176, 66, 72, 211, 235,
                        150, 205, 60, 31, 171, 166, 181, 168, 69, 168, 158, 9, 212, 224, 176, 216,
                        137, 114, 210, 246, 159, 181, 77, 167, 120, 238, 32, 34, 45, 8, 1, 18, 41,
                        28, 232, 52, 234, 218, 174, 23, 32, 188, 249, 183, 129, 112, 210, 118, 157,
                        232, 31, 59, 160, 53, 121, 171, 99, 187, 209, 120, 28, 23, 201, 122, 127,
                        170, 196, 221, 89, 178, 41, 145, 40, 32, 34, 45, 8, 1, 18, 41, 30, 192, 77,
                        234, 218, 174, 23, 32, 35, 220, 125, 199, 215, 104, 26, 234, 89, 248, 27,
                        214, 129, 233, 31, 246, 207, 151, 33, 228, 197, 55, 108, 142, 67, 225, 184,
                        239, 117, 136, 1, 93, 32, 34, 46, 8, 1, 18, 42, 32, 138, 158, 1, 234, 218,
                        174, 23, 32, 235, 104, 70, 104, 120, 174, 178, 48, 39, 190, 220, 254, 169,
                        174, 149, 172, 125, 104, 182, 58, 105, 157, 253, 60, 190, 130, 167, 38, 17,
                        154, 132, 73, 32, 34, 46, 8, 1, 18, 42, 36, 186, 249, 2, 234, 218, 174, 23,
                        32, 165, 175, 18, 209, 243, 50, 151, 238, 140, 138, 130, 218, 36, 78, 133,
                        210, 214, 131, 185, 247, 168, 113, 177, 88, 235, 253, 113, 5, 121, 32, 231,
                        17, 32, 34, 46, 8, 1, 18, 42, 38, 212, 225, 4, 234, 218, 174, 23, 32, 86,
                        199, 34, 112, 105, 207, 250, 67, 127, 43, 169, 12, 142, 123, 143, 51, 207,
                        16, 6, 82, 142, 205, 126, 75, 228, 223, 136, 228, 80, 138, 58, 238, 32, 34,
                        46, 8, 1, 18, 42, 40, 160, 184, 11, 234, 218, 174, 23, 32, 190, 136, 158,
                        166, 49, 9, 22, 49, 242, 138, 126, 189, 78, 0, 44, 235, 145, 95, 60, 29,
                        34, 69, 76, 193, 57, 229, 98, 152, 245, 218, 65, 72, 32, 34, 46, 8, 1, 18,
                        42, 42, 236, 250, 24, 234, 218, 174, 23, 32, 175, 6, 248, 83, 181, 82, 10,
                        62, 53, 225, 77, 30, 204, 172, 53, 81, 66, 23, 45, 200, 113, 40, 123, 29,
                        146, 217, 141, 34, 90, 27, 169, 81, 32, 34, 46, 8, 1, 18, 42, 44, 146, 214,
                        50, 234, 218, 174, 23, 32, 94, 2, 184, 40, 253, 43, 30, 227, 176, 0, 20,
                        94, 188, 243, 20, 250, 0, 217, 98, 175, 212, 250, 34, 30, 75, 85, 183, 110,
                        39, 211, 152, 221, 32, 34, 46, 8, 1, 18, 42, 46, 172, 160, 100, 234, 218,
                        174, 23, 32, 76, 237, 39, 152, 243, 78, 62, 8, 241, 65, 148, 191, 130, 18,
                        128, 201, 6, 100, 142, 183, 135, 66, 22, 60, 180, 175, 115, 110, 101, 169,
                        187, 170, 32, 34, 47, 8, 1, 18, 43, 48, 146, 149, 169, 1, 234, 218, 174,
                        23, 32, 220, 32, 110, 3, 59, 34, 149, 108, 2, 60, 85, 63, 18, 58, 247, 15,
                        122, 121, 196, 186, 1, 64, 141, 52, 191, 158, 118, 157, 90, 152, 151, 10,
                        32, 34, 47, 8, 1, 18, 43, 50, 224, 172, 165, 6, 234, 218, 174, 23, 32, 99,
                        253, 205, 74, 11, 222, 9, 96, 40, 243, 6, 80, 209, 239, 44, 224, 61, 228,
                        219, 46, 203, 110, 7, 198, 44, 187, 7, 188, 34, 232, 134, 89, 32,
                    ],
                },
                ProofOp {
                    field_type: "ics23:simple".to_string(),
                    key: vec![119, 97, 115, 109],
                    data: vec![
                        10, 207, 1, 10, 4, 119, 97, 115, 109, 18, 32, 13, 69, 53, 75, 228, 194,
                        187, 93, 173, 41, 132, 253, 9, 176, 42, 74, 43, 91, 45, 29, 32, 175, 153,
                        200, 197, 151, 4, 228, 158, 86, 247, 213, 26, 9, 8, 1, 24, 1, 32, 1, 42, 1,
                        0, 34, 37, 8, 1, 18, 33, 1, 186, 14, 12, 99, 42, 81, 49, 208, 29, 220, 17,
                        77, 94, 60, 53, 136, 186, 161, 159, 89, 208, 40, 50, 106, 159, 237, 89, 92,
                        4, 104, 92, 5, 34, 37, 8, 1, 18, 33, 1, 1, 61, 56, 223, 94, 160, 6, 240,
                        74, 206, 212, 187, 149, 161, 243, 223, 100, 192, 244, 96, 10, 215, 88, 178,
                        162, 96, 163, 214, 83, 103, 139, 92, 34, 37, 8, 1, 18, 33, 1, 52, 75, 127,
                        225, 113, 14, 109, 134, 230, 172, 246, 250, 42, 187, 199, 132, 148, 242,
                        201, 157, 231, 55, 3, 172, 182, 37, 145, 227, 85, 196, 77, 249, 34, 37, 8,
                        1, 18, 33, 1, 215, 128, 217, 168, 205, 219, 122, 70, 59, 154, 4, 166, 6,
                        139, 215, 93, 48, 11, 247, 115, 216, 104, 102, 153, 250, 77, 126, 220, 206,
                        102, 146, 190,
                    ],
                },
            ],
        };

        let res = ProvableAbciQueryResponse {
            response: vec![
                34, 110, 49, 116, 120, 101, 51, 120, 48, 52, 99, 51, 119, 106, 101, 106, 102, 110,
                52, 109, 121, 52, 101, 102, 51, 110, 52, 116, 52, 100, 102, 104, 102, 97, 106, 99,
                119, 113, 103, 57, 116, 34,
            ],
            height,
            proof,
        };

        // the app_hash committing state at H lives in header[H+1] (CometBFT off-by-one)
        let app_hash = AppHash::try_from(vec![
            77, 168, 114, 14, 213, 137, 50, 46, 92, 222, 67, 61, 187, 195, 144, 206, 164, 177, 231,
            194, 137, 130, 216, 196, 4, 67, 131, 147, 219, 101, 95, 230,
        ])?;

        // reconstruct the raw wasm key: 0x03 || canonical_addr || b"admin" (no length prefix)
        let mut key = vec![0x03u8];
        key.extend_from_slice(&contract.to_bytes());
        key.extend_from_slice(b"admin");
        assert_eq!(
            key, res.proof.ops[0].key,
            "reconstructed raw key must match the proven key"
        );

        // positive: verifies against the correct app_hash
        verify_wasm_store_membership(&res.proof.ops, app_hash.as_bytes(), &key, &res.response)?;

        // negative: a wrong app_hash is rejected at the store layer
        let mut wrong_app_hash = app_hash.as_bytes().to_vec();
        wrong_app_hash[0] ^= 0xff;
        assert!(matches!(
            verify_wasm_store_membership(&res.proof.ops, &wrong_app_hash, &key, &res.response),
            Err(ProofError::StoreVerificationFailed)
        ));

        // negative: a tampered value is rejected at the IAVL layer
        let mut tampered = res.response.clone();
        tampered[5] ^= 0xff;
        assert!(matches!(
            verify_wasm_store_membership(&res.proof.ops, app_hash.as_bytes(), &key, &tampered),
            Err(ProofError::IavlVerificationFailed)
        ));

        Ok(())
    }
}
