// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use cosmwasm_std::{Addr, QuerierWrapper};
use cw4::Cw4Contract;

pub(crate) fn group_members(
    querier_wrapper: &QuerierWrapper,
    contract: &Cw4Contract,
) -> Result<Vec<Addr>, ContractError> {
    // we shouldn't ever have more group members than the default limit but IN CASE
    // something changes down the line, do go through the pagination flow
    let mut group_members = Vec::new();

    // current max limit
    let limit = 30;
    let mut start_after = None;
    loop {
        let members = contract.list_members(querier_wrapper, start_after, Some(limit))?;
        start_after = members.last().as_ref().map(|d| d.addr.clone());
        for member in &members {
            group_members.push(Addr::unchecked(&member.addr));
        }

        if members.len() < limit as usize {
            // we have already exhausted the data
            break;
        }
    }

    Ok(group_members)
}

#[cfg(test)]
mod tests {
    use crate::testable_dkg_contract::helpers::group_members;
    use crate::testable_dkg_contract::init_contract_tester_with_group_members;
    use cw4::Cw4Contract;
    use cw4_group::testable_cw4_contract::GroupContract;
    use nym_contracts_common_testing::ContractOpts;

    #[test]
    fn getting_group_members() -> anyhow::Result<()> {
        for members in [0, 10, 100, 1000] {
            let tester = init_contract_tester_with_group_members(members);
            let group_contract =
                Cw4Contract::new(tester.unchecked_contract_address::<GroupContract>());
            let querier = tester.deps().querier;

            let addresses = group_members(&querier, &group_contract)?;
            assert_eq!(addresses.len(), members);
        }

        Ok(())
    }
}
