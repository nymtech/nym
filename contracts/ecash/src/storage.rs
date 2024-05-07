// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use nym_ecash_contract_common::{
    blacklist::{BlacklistProposal, BlacklistedAccount},
    spend_credential::EcashSpentCredential,
};

// storage prefixes
const SPEND_CREDENTIAL_PK_NAMESPACE: &str = "ecsc";
const SPEND_CREDENTIAL_BLINDED_SERIAL_NO_IDX_NAMESPACE: &str = "ecscn";

// paged retrieval limits for all queries and transactions
pub(crate) const SPEND_CREDENTIAL_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const SPEND_CREDENTIAL_PAGE_DEFAULT_LIMIT: u32 = 50;

pub(crate) struct SpendCredentialIndex<'a> {
    pub(crate) blinded_serial_number: UniqueIndex<'a, String, EcashSpentCredential>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<EcashSpentCredential> for SpendCredentialIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<EcashSpentCredential>> + '_> {
        let v: Vec<&dyn Index<EcashSpentCredential>> = vec![&self.blinded_serial_number];
        Box::new(v.into_iter())
    }
}

// spent_credentials() is the storage access function.
pub(crate) const fn spent_credentials<'a>(
) -> IndexedMap<'a, &'a str, EcashSpentCredential, SpendCredentialIndex<'a>> {
    let indexes = SpendCredentialIndex {
        blinded_serial_number: UniqueIndex::new(
            |d| d.serial_number().to_string(),
            SPEND_CREDENTIAL_BLINDED_SERIAL_NO_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(SPEND_CREDENTIAL_PK_NAMESPACE, indexes)
}

// storage prefixes
const BLACKLIST_PK_NAMESPACE: &str = "blacklist";
const BLACKLIST_NO_IDX_NAMESPACE: &str = "blacklistnoidx";

// paged retrieval limits for all queries and transactions
pub(crate) const BLACKLIST_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const BLACKLIST_PAGE_DEFAULT_LIMIT: u32 = 50;

pub(crate) struct BlacklistIndex<'a> {
    pub(crate) blacklist: UniqueIndex<'a, String, BlacklistedAccount>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<BlacklistedAccount> for BlacklistIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<BlacklistedAccount>> + '_> {
        let v: Vec<&dyn Index<BlacklistedAccount>> = vec![&self.blacklist];
        Box::new(v.into_iter())
    }
}

// spent_credentials() is the storage access function.
pub(crate) const fn blacklist<'a>(
) -> IndexedMap<'a, &'a str, BlacklistedAccount, BlacklistIndex<'a>> {
    let indexes = BlacklistIndex {
        blacklist: UniqueIndex::new(|d| d.public_key().to_string(), BLACKLIST_NO_IDX_NAMESPACE),
    };
    IndexedMap::new(BLACKLIST_PK_NAMESPACE, indexes)
}

// storage prefixes
const BLACKLIST_PROPOSAL_PK_NAMESPACE: &str = "blacklistproposal";
const BLACKLIST_PROPOSAL_NO_IDX_NAMESPACE: &str = "blacklistproposalnoidx";

pub(crate) struct BlacklistProposalIndex<'a> {
    pub(crate) blacklist_proposal: UniqueIndex<'a, String, BlacklistProposal>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<BlacklistProposal> for BlacklistProposalIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<BlacklistProposal>> + '_> {
        let v: Vec<&dyn Index<BlacklistProposal>> = vec![&self.blacklist_proposal];
        Box::new(v.into_iter())
    }
}

// spent_credentials() is the storage access function.
pub(crate) const fn blacklist_proposal<'a>(
) -> IndexedMap<'a, &'a str, BlacklistProposal, BlacklistProposalIndex<'a>> {
    let indexes = BlacklistProposalIndex {
        blacklist_proposal: UniqueIndex::new(
            |d| d.public_key().to_string(),
            BLACKLIST_PROPOSAL_NO_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(BLACKLIST_PROPOSAL_PK_NAMESPACE, indexes)
}
