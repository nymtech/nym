// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Event, MessageInfo, Response};
use nym_coconut_bandwidth_contract_common::spend_credential::{
    to_cosmos_msg, SpendCredential, SpendCredentialData,
};

use crate::error::ContractError;
use crate::state::{ADMIN, CONFIG};
use crate::storage;

use nym_coconut_bandwidth_contract_common::deposit::DepositData;
use nym_coconut_bandwidth_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_IDENTITY_KEY, DEPOSIT_INFO,
    DEPOSIT_VALUE,
};

pub(crate) fn deposit_funds(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    data: DepositData,
) -> Result<Response, ContractError> {
    if info.funds.is_empty() {
        return Err(ContractError::NoCoin);
    }
    if info.funds.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }
    let mix_denom = CONFIG.load(deps.storage)?.mix_denom;
    if info.funds[0].denom != mix_denom {
        return Err(ContractError::WrongDenom { mix_denom });
    }

    let voucher_value = info.funds.last().unwrap();
    let event = Event::new(DEPOSITED_FUNDS_EVENT_TYPE)
        .add_attribute(DEPOSIT_VALUE, voucher_value.amount)
        .add_attribute(DEPOSIT_INFO, data.deposit_info())
        .add_attribute(DEPOSIT_IDENTITY_KEY, data.identity_key())
        .add_attribute(DEPOSIT_ENCRYPTION_KEY, data.encryption_key());

    Ok(Response::new().add_event(event))
}

pub(crate) fn spend_credential(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    data: SpendCredentialData,
) -> Result<Response, ContractError> {
    let mix_denom = CONFIG.load(deps.storage)?.mix_denom;
    if data.funds().denom != mix_denom {
        return Err(ContractError::WrongDenom { mix_denom });
    }
    if storage::spent_credentials().has(deps.storage, data.blinded_serial_number()) {
        return Err(ContractError::DuplicateBlindedSerialNumber);
    }
    let cfg = CONFIG.load(deps.storage)?;

    let gateway_cosmos_address = deps.api.addr_validate(data.gateway_cosmos_address())?;
    storage::spent_credentials().save(
        deps.storage,
        data.blinded_serial_number(),
        &SpendCredential::new(
            data.funds().to_owned(),
            data.blinded_serial_number().to_owned(),
            gateway_cosmos_address,
        ),
    )?;

    let msg = to_cosmos_msg(
        data.funds().clone(),
        data.blinded_serial_number().to_string(),
        env.contract.address.into_string(),
        cfg.multisig_addr.into_string(),
    )?;

    Ok(Response::new().add_message(msg))
}

pub(crate) fn release_funds(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    funds: Coin,
) -> Result<Response, ContractError> {
    let mix_denom = CONFIG.load(deps.storage)?.mix_denom;
    if funds.denom != mix_denom {
        return Err(ContractError::WrongDenom { mix_denom });
    }
    let current_balance = deps
        .querier
        .query_balance(env.contract.address, mix_denom)?;
    if funds.amount > current_balance.amount {
        return Err(ContractError::NotEnoughFunds);
    }
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let cfg = CONFIG.load(deps.storage)?;

    let return_tokens = BankMsg::Send {
        to_address: cfg.pool_addr.into(),
        amount: vec![funds],
    };
    let response = Response::new().add_message(return_tokens);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::fixtures::spend_credential_data_fixture;
    use crate::support::tests::helpers::{self, MULTISIG_CONTRACT, POOL_CONTRACT};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{from_binary, CosmosMsg, WasmMsg};
    use cw_controllers::AdminError;
    use nym_coconut_bandwidth_contract_common::msg::ExecuteMsg;
    use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;

    #[test]
    fn invalid_deposit() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        let deposit_info = String::from("Deposit info");
        let verification_key = String::from("Verification key");
        let encryption_key = String::from("Encryption key");
        let data = DepositData::new(deposit_info, verification_key, encryption_key);

        assert_eq!(
            deposit_funds(deps.as_mut(), env.clone(), info, data.clone()),
            Err(ContractError::NoCoin)
        );

        let coin = Coin::new(1000000, crate::support::tests::fixtures::TEST_MIX_DENOM);
        let second_coin = Coin::new(1000000, "some_denom");

        let info = mock_info("requester", &[coin, second_coin.clone()]);
        assert_eq!(
            deposit_funds(deps.as_mut(), env.clone(), info, data.clone()),
            Err(ContractError::MultipleDenoms)
        );

        let info = mock_info("requester", &[second_coin]);
        assert_eq!(
            deposit_funds(deps.as_mut(), env, info, data),
            Err(ContractError::WrongDenom {
                mix_denom: crate::support::tests::fixtures::TEST_MIX_DENOM.to_string()
            })
        );
    }

    #[test]
    fn valid_deposit() {
        let mut deps = helpers::init_contract();
        let env = mock_env();

        let deposit_info = String::from("Deposit info");
        let verification_key = String::from("Verification key");
        let encryption_key = String::from("Encryption key");
        let deposit_value = 424242;
        let data = DepositData::new(
            deposit_info.clone(),
            verification_key.clone(),
            encryption_key.clone(),
        );
        let coin = Coin::new(
            deposit_value,
            crate::support::tests::fixtures::TEST_MIX_DENOM,
        );
        let info = mock_info("requester", &[coin]);

        let tx = deposit_funds(deps.as_mut(), env, info, data).unwrap();

        let events: Vec<_> = tx
            .events
            .iter()
            .filter(|event| event.ty == DEPOSITED_FUNDS_EVENT_TYPE)
            .collect();
        assert_eq!(events.len(), 1);

        let event = events[0];
        assert_eq!(event.attributes.len(), 4);

        let deposit_attr = event
            .attributes
            .iter()
            .find(|attr| attr.key == DEPOSIT_VALUE)
            .unwrap();
        assert_eq!(deposit_attr.value, deposit_value.to_string());

        let info_attr = event
            .attributes
            .iter()
            .find(|attr| attr.key == DEPOSIT_INFO)
            .unwrap();
        assert_eq!(info_attr.value, deposit_info);

        let verification_key_attr = event
            .attributes
            .iter()
            .find(|attr| attr.key == DEPOSIT_IDENTITY_KEY)
            .unwrap();
        assert_eq!(verification_key_attr.value, verification_key);

        let encryption_key_attr = event
            .attributes
            .iter()
            .find(|attr| attr.key == DEPOSIT_ENCRYPTION_KEY)
            .unwrap();
        assert_eq!(encryption_key_attr.value, encryption_key);
    }

    #[test]
    fn invalid_release() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let invalid_admin = "invalid admin";
        let funds = Coin::new(1, crate::support::tests::fixtures::TEST_MIX_DENOM);

        let err = release_funds(
            deps.as_mut(),
            env.clone(),
            mock_info(invalid_admin, &[]),
            Coin::new(1, "invalid denom"),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::WrongDenom {
                mix_denom: crate::support::tests::fixtures::TEST_MIX_DENOM.to_string()
            }
        );

        let err = release_funds(
            deps.as_mut(),
            env.clone(),
            mock_info(invalid_admin, &[]),
            funds.clone(),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::NotEnoughFunds);

        deps.querier
            .update_balance(env.contract.address.clone(), vec![funds.clone()]);
        let err =
            release_funds(deps.as_mut(), env, mock_info(invalid_admin, &[]), funds).unwrap_err();
        assert_eq!(err, ContractError::Admin(AdminError::NotAdmin {}));
    }

    #[test]
    fn valid_release() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let coin = Coin::new(1, crate::support::tests::fixtures::TEST_MIX_DENOM);

        deps.querier
            .update_balance(env.contract.address.clone(), vec![coin.clone()]);
        let res = release_funds(
            deps.as_mut(),
            env,
            mock_info(MULTISIG_CONTRACT, &[]),
            coin.clone(),
        )
        .unwrap();
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from(POOL_CONTRACT),
                amount: vec![coin]
            })
        );
    }
    #[test]
    fn valid_spend() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        let data = spend_credential_data_fixture("blinded_serial_number");
        let res = spend_credential(deps.as_mut(), env.clone(), info, data.clone()).unwrap();
        if let CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        }) = &res.messages[0].msg
        {
            assert_eq!(contract_addr, MULTISIG_CONTRACT);
            assert!(funds.is_empty());
            let multisig_msg: MultisigExecuteMsg = from_binary(msg).unwrap();
            if let MultisigExecuteMsg::Propose {
                title: _,
                description,
                msgs,
                latest,
            } = multisig_msg
            {
                assert_eq!(description, data.blinded_serial_number().to_string());
                assert!(latest.is_none());
                if let CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg,
                    funds,
                }) = &msgs[0]
                {
                    assert_eq!(*contract_addr, env.contract.address.into_string());
                    assert!(funds.is_empty());
                    let release_funds_req: ExecuteMsg = from_binary(msg).unwrap();
                    if let ExecuteMsg::ReleaseFunds { funds } = release_funds_req {
                        assert_eq!(funds, *data.funds());
                    } else {
                        panic!("Could not extract release funds message from proposal");
                    }
                }
            } else {
                panic!("Could not extract proposal from binary blob");
            }
        } else {
            panic!("Wasm execute message not found");
        }
    }

    #[test]
    fn invalid_spend_attempts() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        let invalid_data = SpendCredentialData::new(
            Coin::new(1, "invalid_denom".to_string()),
            String::new(),
            String::new(),
        );
        let ret = spend_credential(deps.as_mut(), env.clone(), info.clone(), invalid_data);
        assert_eq!(
            ret.unwrap_err(),
            ContractError::WrongDenom {
                mix_denom: crate::support::tests::fixtures::TEST_MIX_DENOM.to_string()
            }
        );

        let invalid_data = SpendCredentialData::new(
            Coin::new(1, crate::support::tests::fixtures::TEST_MIX_DENOM),
            String::new(),
            "Blinded Serial Number".to_string(),
        );
        let ret = spend_credential(deps.as_mut(), env.clone(), info.clone(), invalid_data);
        assert_eq!(
            ret.unwrap_err().to_string(),
            "Generic error: Invalid input: address not normalized".to_string()
        );

        let invalid_data = spend_credential_data_fixture("blined_serial_number");
        spend_credential(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            invalid_data.clone(),
        )
        .unwrap();
        let ret = spend_credential(deps.as_mut(), env, info, invalid_data);
        assert_eq!(
            ret.unwrap_err(),
            ContractError::DuplicateBlindedSerialNumber
        );
    }
}
