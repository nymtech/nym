// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::state::storage::DKG_ADMIN;
use cosmwasm_std::{DepsMut, MessageInfo, Response};

pub(crate) fn try_update_admin(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    Ok(DKG_ADMIN.execute_update_admin(deps, info, Some(new_admin))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::Addr;
    use cw_controllers::AdminError;
    use nym_coconut_dkg_common::msg::ExecuteMsg;

    fn update_admin_msg(admin: &Addr) -> ExecuteMsg {
        ExecuteMsg::UpdateAdmin {
            admin: admin.to_string(),
        }
    }

    #[test]
    fn can_only_be_performed_by_current_admin() {
        let mut deps = init_contract();
        let env = mock_env();
        let old_admin = Addr::unchecked(ADMIN_ADDRESS);
        let not_admin = deps.api.addr_make("not an admin");
        let new_admin = deps.api.addr_make("new admin");

        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&not_admin, &[]),
            update_admin_msg(&new_admin),
        )
        .unwrap_err();
        assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);
        assert!(DKG_ADMIN.is_admin(deps.as_ref(), &old_admin).unwrap());

        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&old_admin, &[]),
            update_admin_msg(&new_admin),
        );
        assert!(res.is_ok());
        assert!(DKG_ADMIN.is_admin(deps.as_ref(), &new_admin).unwrap());

        // the previous admin no longer holds the role
        let res = execute(
            deps.as_mut(),
            env,
            message_info(&old_admin, &[]),
            update_admin_msg(&old_admin),
        )
        .unwrap_err();
        assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);
    }

    #[test]
    fn requires_providing_valid_address() {
        let mut deps = init_contract();
        let admin = Addr::unchecked(ADMIN_ADDRESS);

        for bad_account in ["definitely-not-valid-account", ""] {
            let res = try_update_admin(
                deps.as_mut(),
                message_info(&admin, &[]),
                bad_account.to_string(),
            );
            assert!(res.is_err());
        }

        assert!(DKG_ADMIN.is_admin(deps.as_ref(), &admin).unwrap());
    }
}
