/*!
This is a basic implementation of the [cw4 spec](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw4/README.md).
It fulfills all elements of the spec, including the raw query lookups,
and it designed to be used as a backing storage for
[cw3 compliant contracts](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw3/README.md).

It stores a set of members along with an admin, and allows the admin to
update the state. Raw queries (intended for cross-contract queries)
can check a given member address and the total weight. Smart queries (designed
for client API) can do the same, and also query the admin address as well as
paginate over all members.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw4-group/README.md).
*/

pub mod contract;
pub mod error;
pub mod helpers;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;

#[cfg(test)]
mod tests;
