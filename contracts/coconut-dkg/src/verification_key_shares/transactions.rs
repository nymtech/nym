// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::BLOCK_TIME_FOR_VERIFICATION_SECS;
use crate::dealers::storage as dealers_storage;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::{MULTISIG, STATE};
use crate::verification_key_shares::storage::VK_SHARES;
use coconut_dkg_common::types::EpochState;
use coconut_dkg_common::verification_key::{to_cosmos_msg, ContractVKShare, VerificationKeyShare};
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

pub fn try_commit_verification_key_share(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    share: VerificationKeyShare,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::VerificationKeySubmission)?;
    // ensure the sender is a dealer
    let details = dealers_storage::current_dealers()
        .load(deps.storage, &info.sender)
        .map_err(|_| ContractError::NotADealer)?;
    if VK_SHARES.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::AlreadyCommitted {
            commitment: String::from("verification key share"),
        });
    }

    let data = ContractVKShare {
        share,
        node_index: details.assigned_index,
        announce_address: details.announce_address,
        owner: info.sender.clone(),
        verified: false,
    };
    VK_SHARES.save(deps.storage, &info.sender, &data)?;

    let msg = to_cosmos_msg(
        info.sender,
        env.contract.address.to_string(),
        STATE.load(deps.storage)?.multisig_addr.to_string(),
        env.block
            .time
            .plus_seconds(BLOCK_TIME_FOR_VERIFICATION_SECS),
    )?;

    Ok(Response::new().add_message(msg))
}

pub fn try_verify_verification_key_share(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: Addr,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::VerificationKeyFinalization)?;
    MULTISIG.assert_admin(deps.as_ref(), &info.sender)?;
    VK_SHARES.update(deps.storage, &owner, |vk_share| {
        vk_share
            .map(|mut share| {
                share.verified = true;
                share
            })
            .ok_or(ContractError::NoCommitForOwner {
                owner: owner.to_string(),
            })
    })?;

    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_state::transactions::advance_epoch_state;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{ADMIN_ADDRESS, MULTISIG_CONTRACT};
    use coconut_dkg_common::dealer::DealerDetails;
    use coconut_dkg_common::types::EpochState;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cw_controllers::AdminError;

    #[test]
    fn commit_vk_share() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        let share = "share".to_string();

        let ret = try_commit_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            share.clone(),
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::default().to_string(),
                expected_state: EpochState::VerificationKeySubmission.to_string()
            }
        );
        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        let ret = try_commit_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            share.clone(),
        )
        .unwrap_err();
        assert_eq!(ret, ContractError::NotADealer);

        let dealer = Addr::unchecked("requester");
        let dealer_details = DealerDetails {
            address: dealer.clone(),
            bte_public_key_with_proof: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        dealers_storage::current_dealers()
            .save(deps.as_mut().storage, &dealer, &dealer_details)
            .unwrap();

        try_commit_verification_key_share(deps.as_mut(), env.clone(), info.clone(), share.clone())
            .unwrap();

        let ret = try_commit_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            share.clone(),
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::AlreadyCommitted {
                commitment: String::from("verification key share")
            }
        );
    }

    #[test]
    fn invalid_verify_vk_share() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        let owner = Addr::unchecked("owner");
        let multisig_info = mock_info(MULTISIG_CONTRACT, &[]);

        let ret = try_verify_verification_key_share(deps.as_mut(), info.clone(), owner.clone())
            .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::default().to_string(),
                expected_state: EpochState::VerificationKeyFinalization.to_string()
            }
        );

        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        advance_epoch_state(deps.as_mut(), env, mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let ret =
            try_verify_verification_key_share(deps.as_mut(), info, owner.clone()).unwrap_err();
        assert_eq!(ret, ContractError::Admin(AdminError::NotAdmin {}));

        let ret = try_verify_verification_key_share(deps.as_mut(), multisig_info, owner.clone())
            .unwrap_err();
        assert_eq!(
            ret,
            ContractError::NoCommitForOwner {
                owner: owner.to_string()
            }
        );
    }

    #[test]
    fn verify_vk_share() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let owner = Addr::unchecked("owner");
        let info = mock_info(owner.as_ref(), &[]);
        let share = "share".to_string();
        let multisig_info = mock_info(MULTISIG_CONTRACT, &[]);

        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let dealer_details = DealerDetails {
            address: owner.clone(),
            bte_public_key_with_proof: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        dealers_storage::current_dealers()
            .save(deps.as_mut().storage, &owner, &dealer_details)
            .unwrap();
        try_commit_verification_key_share(deps.as_mut(), env.clone(), info.clone(), share.clone())
            .unwrap();

        advance_epoch_state(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();
        advance_epoch_state(deps.as_mut(), env, mock_info(ADMIN_ADDRESS, &[])).unwrap();

        try_verify_verification_key_share(deps.as_mut(), multisig_info, owner.clone()).unwrap();
    }
}
