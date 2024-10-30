// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use nym_credentials_interface::TicketType;
use rs_merkle::algorithms::Sha256;
use rs_merkle::{MerkleProof, MerkleTree};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use time::Date;

// no point in importing the entire contract commons just for this one type
pub type DepositId = u32;
pub type DKGEpochId = u64;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedTicketbook {
    pub deposit_id: DepositId,
    pub epoch_id: DKGEpochId,

    // 96 bytes serialised 'BlindedSignature'
    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::base64")]
    pub blinded_partial_credential: Vec<u8>,

    // concatenated bytes for the commitments to the private attributes
    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::base64")]
    pub joined_encoded_private_attributes_commitments: Vec<u8>,

    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::date")]
    pub expiration_date: Date,

    #[schemars(with = "String")]
    pub ticketbook_type: TicketType,
}

impl IssuedTicketbook {
    pub fn hash_to_merkle_leaf(&self) -> [u8; 32] {
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.deposit_id.to_be_bytes());
        hasher.update(self.epoch_id.to_be_bytes());
        hasher.update(&self.blinded_partial_credential);
        hasher.update(&self.joined_encoded_private_attributes_commitments);
        hasher.update(self.expiration_date.to_julian_day().to_be_bytes());
        hasher.update(self.ticketbook_type.encode().to_be_bytes());

        hasher.finalize().into()
    }

    pub fn signable_plaintext(&self) -> Vec<u8> {
        self.deposit_id
            .to_be_bytes()
            .into_iter()
            .chain(self.epoch_id.to_be_bytes())
            .chain(self.blinded_partial_credential.iter().copied())
            .chain(
                self.joined_encoded_private_attributes_commitments
                    .iter()
                    .copied(),
            )
            .chain(self.expiration_date.to_julian_day().to_be_bytes())
            .chain(self.ticketbook_type.encode().to_be_bytes())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertedMerkleLeaf {
    #[serde(with = "nym_serde_helpers::hex")]
    pub new_root: Vec<u8>,
    pub leaf: MerkleLeaf,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialOrd, PartialEq, Eq)]
pub struct MerkleLeaf {
    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::hex")]
    pub hash: Vec<u8>,
    pub index: usize,
}

#[derive(Default, Clone)]
pub struct IssuedTicketbooksMerkleTree {
    inner: MerkleTree<Sha256>,
}

impl IssuedTicketbooksMerkleTree {
    pub fn new() -> IssuedTicketbooksMerkleTree {
        IssuedTicketbooksMerkleTree {
            inner: MerkleTree::new(),
        }
    }

    pub fn rebuild(leaves: &[[u8; 32]]) -> IssuedTicketbooksMerkleTree {
        IssuedTicketbooksMerkleTree {
            inner: MerkleTree::from_leaves(leaves),
        }
    }

    pub fn all_leaves(&self) -> Option<Vec<[u8; 32]>> {
        self.inner.leaves()
    }

    pub fn insert(&mut self, issued: &IssuedTicketbook) -> InsertedMerkleLeaf {
        let hash = issued.hash_to_merkle_leaf();
        self.insert_leaf(hash)
    }

    #[allow(clippy::unwrap_used)]
    pub fn insert_leaf(&mut self, leaf_hash: [u8; 32]) -> InsertedMerkleLeaf {
        let leaves = self.inner.leaves_len();
        self.inner.insert(leaf_hash).commit();

        InsertedMerkleLeaf {
            // SAFETY: after inserting at least a single node, the root will always be available
            new_root: self.inner.root().unwrap().to_vec(),
            leaf: MerkleLeaf {
                hash: leaf_hash.to_vec(),
                index: leaves,
            },
        }
    }

    pub fn rollback(&mut self) {
        self.inner.rollback();
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.inner.root()
    }

    pub fn generate_proof(
        &self,
        leaf_indices: &[usize],
    ) -> Option<IssuedTicketbooksFullMerkleProof> {
        let leaves = self.inner.leaves()?;

        let mut included_leaves = Vec::new();
        for &index in leaf_indices {
            let hash = *leaves.get(index)?;
            included_leaves.push(MerkleLeaf {
                hash: hash.to_vec(),
                index,
            })
        }

        Some(IssuedTicketbooksFullMerkleProof {
            inner_proof: self.inner.proof(leaf_indices),
            included_leaves,
            total_leaves: self.inner.leaves_len(),
            root: self.inner.root()?.to_vec(),
        })
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct IssuedTicketbooksFullMerkleProof {
    #[schemars(with = "String")]
    #[serde(with = "inner_proof_base64_serde")]
    inner_proof: MerkleProof<Sha256>,

    included_leaves: Vec<MerkleLeaf>,

    total_leaves: usize,

    #[schemars(with = "String")]
    #[serde(with = "nym_serde_helpers::hex")]
    root: Vec<u8>,
}

impl IssuedTicketbooksFullMerkleProof {
    pub fn contains_leaf(&self, hash: [u8; 32]) -> bool {
        self.included_leaves.iter().any(|m| m.hash == hash)
    }

    pub fn verify(&self, expected_root: [u8; 32]) -> bool {
        if self.root != expected_root {
            return false;
        }

        let mut leaf_indices = Vec::with_capacity(self.included_leaves.len());
        let mut leaf_hashes = Vec::with_capacity(self.included_leaves.len());
        for leaf in &self.included_leaves {
            leaf_indices.push(leaf.index);
            let Ok(sha256_hash) = leaf.hash.clone().try_into() else {
                return false;
            };
            leaf_hashes.push(sha256_hash);
        }

        self.inner_proof.verify(
            expected_root,
            &leaf_indices,
            &leaf_hashes,
            self.total_leaves,
        )
    }
}

mod inner_proof_base64_serde {
    use rs_merkle::algorithms::Sha256;
    use rs_merkle::proof_serializers::DirectHashesOrder;
    use rs_merkle::MerkleProof;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        proof: &MerkleProof<Sha256>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes = proof.serialize::<DirectHashesOrder>();
        nym_serde_helpers::base64::serialize(&bytes, serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<MerkleProof<Sha256>, D::Error> {
        let bytes = nym_serde_helpers::base64::deserialize(deserializer)?;
        MerkleProof::<Sha256>::deserialize::<DirectHashesOrder>(&bytes)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_credentials_interface::ecash_today;
    use rand::{RngCore, SeedableRng};

    fn test_rng() -> rand_chacha::ChaChaRng {
        let dummy_seed = [42u8; 32];
        rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
    }

    fn dummy_issued(rng: &mut rand_chacha::ChaCha20Rng) -> IssuedTicketbook {
        let mut blinded_partial_credential = vec![0u8; 42];
        rng.fill_bytes(&mut blinded_partial_credential);

        let mut joined_encoded_private_attributes_commitments = vec![0u8; 48 * 3];
        rng.fill_bytes(&mut joined_encoded_private_attributes_commitments);

        IssuedTicketbook {
            deposit_id: rng.next_u32(),
            epoch_id: rng.next_u64(),
            blinded_partial_credential,
            joined_encoded_private_attributes_commitments,
            expiration_date: ecash_today().date(),
            ticketbook_type: TicketType::V1MixnetEntry,
        }
    }

    #[test]
    fn single_leaf() {
        let mut rng = test_rng();
        let issued = dummy_issued(&mut rng);
        let expected_hash = issued.hash_to_merkle_leaf();

        let mut tree = IssuedTicketbooksMerkleTree::new();
        let inserted_node = tree.insert(&issued);

        assert_eq!(inserted_node.leaf.index, 0);
        assert_eq!(inserted_node.leaf.hash, expected_hash);
        assert_eq!(inserted_node.new_root, expected_hash);

        let proof = tree.generate_proof(&[0]).unwrap();
        assert!(proof.verify(expected_hash));
        assert_eq!(proof.total_leaves, 1);
        assert_eq!(proof.included_leaves, vec![inserted_node.leaf]);
        assert_eq!(proof.root, expected_hash);
    }

    #[test]
    fn multiple_leaves() {
        let mut rng = test_rng();
        let mut tree = IssuedTicketbooksMerkleTree::new();

        for i in 0..100 {
            let issued = dummy_issued(&mut rng);
            let expected_hash = issued.hash_to_merkle_leaf();

            let inserted_node = tree.insert(&issued);

            assert_eq!(inserted_node.leaf.index, i);
            assert_eq!(inserted_node.leaf.hash, expected_hash);

            // proof for this single node
            let proof = tree.generate_proof(&[i]).unwrap();
            assert!(proof.verify(tree.root().unwrap()));
            assert_eq!(proof.total_leaves, i + 1);
            assert!(proof.contains_leaf(expected_hash));
        }

        // proof for multiple nodes
        let indices = [0, 5, 42, 69, 74, 99];
        let all_leaves = tree.inner.leaves().unwrap();
        let big_proof = tree.generate_proof(&indices).unwrap();
        for &index in &indices {
            let leaf_hash = all_leaves.get(index).unwrap();
            assert!(big_proof.contains_leaf(*leaf_hash));
        }

        assert!(big_proof.verify(tree.root().unwrap()))
    }

    #[test]
    fn merkle_proof_serialisation_roundtrip() {
        let mut rng = test_rng();
        let mut tree = IssuedTicketbooksMerkleTree::new();

        for _ in 0..100 {
            let issued = dummy_issued(&mut rng);
            tree.insert(&issued);
        }

        let indices = [0, 5, 42, 69, 74, 99];
        let big_proof = tree.generate_proof(&indices).unwrap();
        let bytes = serde_json::to_vec(&big_proof).unwrap();

        let recovered: IssuedTicketbooksFullMerkleProof = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            big_proof.inner_proof.proof_hashes(),
            recovered.inner_proof.proof_hashes()
        );
        assert_eq!(big_proof.included_leaves, recovered.included_leaves);
        assert_eq!(big_proof.total_leaves, recovered.total_leaves);
        assert_eq!(big_proof.root, recovered.root);
    }
}
