use cosmwasm_std::Uint128;
use mixnet_contract_common::Coin;

pub fn unym_to_nym(unym: Coin, fallback_amount: Option<u128>) -> Coin {
    let mut nym = unym;
    nym.amount = nym
        .amount
        .checked_div(Uint128::from(1_000_000_u128))
        .unwrap_or_else(|e| {
            warn!("Fail to convert unym to nym: {}", e);
            if let Some(v) = fallback_amount {
                Uint128::from(v)
            } else {
                Uint128::from(0_u128)
            }
        });
    nym.denom = "nym".into();
    nym
}
