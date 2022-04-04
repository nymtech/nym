// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Event, MessageInfo, Response};

use crate::error::ContractError;
use crate::{ADMIN, CONFIG};

use coconut_bandwidth_contract_common::deposit::DepositData;
use coconut_bandwidth_contract_common::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_IDENTITY_KEY, DEPOSIT_INFO,
    DEPOSIT_VALUE,
};
use config::defaults::DENOM;

pub(crate) fn deposit_funds(
    _deps: DepsMut<'_>,
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
    if info.funds[0].denom != DENOM {
        return Err(ContractError::WrongDenom);
    }

    let voucher_value = info.funds.last().unwrap();
    let event = Event::new(DEPOSITED_FUNDS_EVENT_TYPE)
        .add_attribute(DEPOSIT_VALUE, voucher_value.amount)
        .add_attribute(DEPOSIT_INFO, data.deposit_info())
        .add_attribute(DEPOSIT_IDENTITY_KEY, data.identity_key())
        .add_attribute(DEPOSIT_ENCRYPTION_KEY, data.encryption_key());

    Ok(Response::new().add_event(event))
}

pub(crate) fn release_funds(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let cfg = CONFIG.load(deps.storage)?;

    let return_tokens = BankMsg::Send {
        to_address: cfg.pool_addr.into(),
        amount: funds,
    };
    let response = Response::new().add_message(return_tokens);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Coin;

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

        let coin = Coin::new(1000000, DENOM);
        let second_coin = Coin::new(1000000, "some_denom");

        let info = mock_info("requester", &[coin, second_coin.clone()]);
        assert_eq!(
            deposit_funds(deps.as_mut(), env.clone(), info, data.clone()),
            Err(ContractError::MultipleDenoms)
        );

        let info = mock_info("requester", &[second_coin]);
        assert_eq!(
            deposit_funds(deps.as_mut(), env, info, data),
            Err(ContractError::WrongDenom)
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
        let coin = Coin::new(deposit_value, DENOM);
        let info = mock_info("requester", &[coin]);

        let tx = deposit_funds(deps.as_mut(), env.clone(), info, data).unwrap();

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
}
