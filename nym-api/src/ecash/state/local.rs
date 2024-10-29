// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::helpers::{
    CachedImmutableEpochItem, CachedImmutableItems, IssuedCoinIndicesSignatures,
    IssuedExpirationDateSignatures,
};
use crate::ecash::keys::KeyPair;
use crate::ecash::storage::models::IssuedHash;
use nym_api_requests::ecash::models::DepositId;
use nym_config::defaults::BloomfilterParameters;
use nym_crypto::asymmetric::identity;
use nym_ecash_double_spending::DoubleSpendingFilter;
use nym_ticketbooks_merkle::{IssuedTicketbooksMerkleTree, MerkleLeaf};
use std::collections::HashMap;
use std::sync::Arc;
use time::Date;
use tokio::sync::RwLock;
use tracing::error;

pub(crate) struct TicketDoubleSpendingFilter {
    built_on: Date,
    params_id: i64,

    today_filter: DoubleSpendingFilter,
    global_filter: DoubleSpendingFilter,
}

impl TicketDoubleSpendingFilter {
    pub(crate) fn new(
        built_on: Date,
        params_id: i64,
        global_filter: DoubleSpendingFilter,
        today_filter: DoubleSpendingFilter,
    ) -> TicketDoubleSpendingFilter {
        TicketDoubleSpendingFilter {
            built_on,
            params_id,
            today_filter,
            global_filter,
        }
    }

    pub(crate) fn built_on(&self) -> Date {
        self.built_on
    }

    pub(crate) fn params(&self) -> BloomfilterParameters {
        self.today_filter.params()
    }

    pub(crate) fn params_id(&self) -> i64 {
        self.params_id
    }

    pub(crate) fn check(&self, sn: &Vec<u8>) -> bool {
        self.global_filter.check(sn)
    }

    /// Returns boolean to indicate if the entry was already present
    pub(crate) fn insert_both(&mut self, sn: &Vec<u8>) -> bool {
        self.today_filter.set(sn);
        self.insert_global_only(sn)
    }

    /// Returns boolean to indicate if the entry was already present
    pub(crate) fn insert_global_only(&mut self, sn: &Vec<u8>) -> bool {
        let existed = self.global_filter.check(sn);
        self.global_filter.set(sn);
        existed
    }

    pub(crate) fn export_today_bitmap(&self) -> Vec<u8> {
        self.today_filter.dump_bitmap()
    }

    pub(crate) fn advance_day(&mut self, date: Date, new_global: DoubleSpendingFilter) {
        self.built_on = date;
        self.global_filter = new_global;
        self.today_filter.reset();
    }
}

#[derive(Default)]
pub(crate) struct DailyMerkleTree {
    pub(crate) merkle_tree: IssuedTicketbooksMerkleTree,
    // keep the individual leaves so that we could easily obtain indices for particular leaves
    // when constructing proofs
    pub(crate) inserted_leaves: HashMap<DepositId, MerkleLeaf>,
}

impl DailyMerkleTree {
    pub(crate) fn new(initial_leaves: Vec<IssuedHash>) -> Self {
        let hashes = initial_leaves
            .iter()
            .map(|i| i.merkle_leaf)
            .collect::<Vec<_>>();

        DailyMerkleTree {
            merkle_tree: IssuedTicketbooksMerkleTree::rebuild(&hashes),
            inserted_leaves: initial_leaves
                .into_iter()
                .enumerate()
                .map(|(index, i)| {
                    (
                        i.deposit_id,
                        MerkleLeaf {
                            index,
                            hash: i.merkle_leaf.to_vec(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn merkle_root(&self) -> Option<[u8; 32]> {
        self.merkle_tree.root()
    }

    pub(crate) fn deposits(&self) -> Vec<DepositId> {
        self.inserted_leaves.keys().copied().collect()
    }

    fn rebuild_without_history(&mut self) {
        let new_tree = if let Some(raw_leaves) = self.merkle_tree.all_leaves() {
            IssuedTicketbooksMerkleTree::rebuild(&raw_leaves)
        } else {
            error!("the merkle tree does not seem to have any leaves for rebuilding!");
            return;
        };
        self.merkle_tree = new_tree;
    }

    pub(crate) fn insert(&mut self, deposit_id: DepositId, leaf: [u8; 32]) {
        let inserted = self.merkle_tree.insert_leaf(leaf);

        // every 1000 leaves, rebuild the tree to purge the history
        // (I wish the API of the library allowed to do it without having to go through those extra steps...)
        if inserted.leaf.index != 0 && inserted.leaf.index % 1000 == 0 {
            self.rebuild_without_history();
        }

        self.inserted_leaves.insert(deposit_id, inserted.leaf);
    }
}

pub(crate) struct LocalEcashState {
    pub(crate) ecash_keypair: KeyPair,
    pub(crate) identity_keypair: identity::KeyPair,

    pub(crate) explicitly_disabled: bool,

    /// Specifies whether this api is a signer in given epoch
    pub(crate) active_signer: CachedImmutableEpochItem<bool>,

    pub(crate) partial_coin_index_signatures: CachedImmutableEpochItem<IssuedCoinIndicesSignatures>,
    pub(crate) partial_expiration_date_signatures:
        CachedImmutableItems<Date, IssuedExpirationDateSignatures>,

    // the actual, up to date, bloomfilter
    pub(crate) double_spending_filter: Arc<RwLock<TicketDoubleSpendingFilter>>,

    // merkle trees for ticketbooks issued for particular expiration dates
    pub(crate) issued_merkle_trees: Arc<RwLock<HashMap<Date, DailyMerkleTree>>>,
}

impl LocalEcashState {
    pub(crate) fn new(
        ecash_keypair: KeyPair,
        identity_keypair: identity::KeyPair,
        double_spending_filter: TicketDoubleSpendingFilter,
        explicitly_disabled: bool,
    ) -> Self {
        LocalEcashState {
            ecash_keypair,
            identity_keypair,
            explicitly_disabled,
            active_signer: Default::default(),
            partial_coin_index_signatures: Default::default(),
            partial_expiration_date_signatures: Default::default(),
            double_spending_filter: Arc::new(RwLock::new(double_spending_filter)),
            issued_merkle_trees: Arc::new(Default::default()),
        }
    }

    pub(crate) async fn is_merkle_empty(&self, expiration_date: Date) -> bool {
        self.issued_merkle_trees
            .read()
            .await
            .get(&expiration_date)
            .is_none()
    }

    pub(crate) async fn insert_issued_ticketbook(
        &self,
        expiration_date: Date,
        deposit_id: DepositId,
        leaf_hash: [u8; 32],
    ) {
        match self
            .issued_merkle_trees
            .write()
            .await
            .get_mut(&expiration_date)
        {
            Some(tree) => tree.insert(deposit_id, leaf_hash),
            None => {
                // that's a serious issue because it means we might be producing invalid merkle tree
                // and thus be producing invalid proofs and as a result accused of cheating the rewarding
                error!("CRITICAL: the merkle tree for {expiration_date} does not exist!");
            }
        }
    }

    pub(crate) async fn remove_old_merkle_trees(&self, current_expiration: Date) {
        todo!("we probably need to keep them for few days (alongside the actual db data) in case rewarder is running behind")
    }
}
