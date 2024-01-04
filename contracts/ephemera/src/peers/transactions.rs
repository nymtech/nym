// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::peers::storage::PEERS;
use crate::state::storage::STATE;
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use nym_ephemera_common::types::JsonPeerInfo;

pub fn try_register_peer(
    deps: DepsMut<'_>,
    info: MessageInfo,
    peer_info: JsonPeerInfo,
) -> Result<Response, ContractError> {
    if PEERS.may_load(deps.storage, info.sender.clone())?.is_none() {
        if STATE
            .load(deps.storage)?
            .group_addr
            .is_voting_member(&deps.querier, &info.sender, None)?
            .is_some()
        {
            PEERS.save(deps.storage, info.sender, &peer_info)?;
            Ok(Default::default())
        } else {
            Err(ContractError::Unauthorized {})
        }
    } else {
        Err(ContractError::AlreadyRegistered)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::fixtures::peer_fixture;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::GROUP_MEMBERS;
    use cosmwasm_std::testing::mock_info;
    use cw4::Member;

    #[test]
    fn peer_registration() {
        let mut deps = helpers::init_contract();
        let peer_info = peer_fixture("owner");
        let info = mock_info("owner", &[]);

        let ret = try_register_peer(deps.as_mut(), info.clone(), peer_info.clone()).unwrap_err();
        assert_eq!(ret, ContractError::Unauthorized);

        GROUP_MEMBERS.lock().unwrap().push((
            Member {
                addr: "owner".to_string(),
                weight: 10,
            },
            1,
        ));

        try_register_peer(deps.as_mut(), info.clone(), peer_info.clone()).unwrap();

        let ret = try_register_peer(deps.as_mut(), info, peer_info).unwrap_err();
        assert_eq!(ret, ContractError::AlreadyRegistered);
    }
}
