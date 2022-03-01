// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod constants;
pub mod contract;
mod error;
mod queries;
mod storage;
mod transactions;

pub(crate) type BlockHeight = u64;
pub(crate) type Epoch = u64;

/* TODOs / open questions:
   - should it be possible to update secure channel public keys?
   - threshold update
   - recreating the entire threshold key
   - initial secret sharing - how many do we share, etc

*/

/*
   Initial flow:
       1. Contract genesis (i.e. init) happens, initial exchange height `H` is defined.
       2. Before `H` is reached, all validators(issuers) submit their secure channel public keys
       3. Once `H` is reached one of the following happens:
           - if less than threshold number of keys is submitted, we wait until threshold is reached
           - else any subsequent public key submissions are 'pushed' onto the next epoch for next resharing event; set is "locked"
       4. All present issuers start submitting their partial shares
       5. After threshold number of shares is received by each party, they start submitting their partial verification keys
       6. (????) UNKNOWN: somehow/somebody submits the 'master' verification key. Perhaps via multi sig?
*/
