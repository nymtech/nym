/*!
This builds on [`cw3_fixed_multisig`] with a more
powerful implementation of the [cw3 spec](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw3/README.md).
It is a multisig contract that is backed by a
[cw4 (group)](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw4/README.md) contract, which independently
maintains the voter set.

This provides 2 main advantages:

* You can create two different multisigs with different voting thresholds
  backed by the same group. Thus, you can have a 50% vote, and a 67% vote
  that always use the same voter set, but can take other actions.
* TODO: It allows dynamic multisig groups.


In addition to the dynamic voting set, the main difference with the native
Cosmos SDK multisig, is that it aggregates the signatures on chain, with
visible proposals (like `x/gov` in the Cosmos SDK), rather than requiring
signers to share signatures off chain.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw3-flex-multisig/README.md).
*/

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
