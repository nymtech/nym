use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::Fee;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/stateparams.ts"))]
#[derive(Serialize, Deserialize)]
pub struct TauriContractStateParams {
    minimum_mixnode_pledge: String,
    minimum_gateway_pledge: String,
    mixnode_rewarded_set_size: u32,
    mixnode_active_set_size: u32,
    staking_supply: String,
}

impl From<ContractStateParams> for TauriContractStateParams {
    fn from(p: ContractStateParams) -> TauriContractStateParams {
        TauriContractStateParams {
            minimum_mixnode_pledge: p.minimum_mixnode_pledge.to_string(),
            minimum_gateway_pledge: p.minimum_gateway_pledge.to_string(),
            mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
            mixnode_active_set_size: p.mixnode_active_set_size,
            staking_supply: p.staking_supply.to_string(),
        }
    }
}

use mixnet_contract_common::ContractStateParams;
use nym_wallet_types::admin::TauriContractStateParams;
use validator_client::nymd::Fee;

    fn try_from(p: TauriContractStateParams) -> Result<ContractStateParams, Self::Error> {
        Ok(ContractStateParams {
            minimum_mixnode_pledge: Uint128::try_from(p.minimum_mixnode_pledge.as_str())?,
            minimum_gateway_pledge: Uint128::try_from(p.minimum_gateway_pledge.as_str())?,
            mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
            mixnode_active_set_size: p.mixnode_active_set_size,
            staking_supply: Uint128::try_from(p.staking_supply.as_str())?,
        })
    }
}

#[tauri::command]
pub async fn get_contract_settings(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractStateParams, BackendError> {
    log::info!(">>> Getting contract settings");
    let res = nymd_client!(state).get_contract_settings().await?.into();
    log::trace!("<<< {:?}", res);
    Ok(res)
}

#[tauri::command]
pub async fn update_contract_settings(
    params: TauriContractStateParams,
    fee: Option<Fee>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriContractStateParams, BackendError> {
    let mixnet_contract_settings_params: ContractStateParams = params.try_into()?;
    log::info!(
        ">>> Updating contract settings: {:?}",
        mixnet_contract_settings_params
    );
    nymd_client!(state)
        .update_contract_settings(mixnet_contract_settings_params.clone(), fee)
        .await?;
    let res = mixnet_contract_settings_params.into();
    log::trace!("<<< {:?}", res);
    Ok(res)
}
