use nym_api_requests::models::{
    GatewayCoreStatusResponse, InclusionProbabilityResponse, MixnodeCoreStatusResponse,
    MixnodeStatus, MixnodeStatusResponse, RewardEstimationResponse, SelectionChance,
    StakeSaturationResponse,
};
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{
    GatewayConfigUpdate, Interval as ContractInterval, IntervalRewardParams,
    IntervalRewardingParamsUpdate, MixNode, MixNodeConfigUpdate, RewardedSetNodeStatus,
    RewardingParams, UnbondedMixnode,
};
use nym_types::account::{Account, AccountEntry, AccountWithMnemonic, Balance};
use nym_types::currency::{CurrencyDenom, DecCoin};
use nym_types::delegation::{
    Delegation, DelegationResult, DelegationWithEverything, DelegationsSummaryResponse,
};
use nym_types::deprecated::{DelegationEvent, DelegationEventKind, WrappedDelegationEvent};
use nym_types::fees::{self, FeeDetails};
use nym_types::gas::{Gas, GasInfo};
use nym_types::gateway::{Gateway, GatewayBond};
use nym_types::mixnode::{MixNodeBond, MixNodeCostParams, MixNodeDetails, MixNodeRewarding};
use nym_types::pending_events::{
    PendingEpochEvent, PendingEpochEventData, PendingIntervalEvent, PendingIntervalEventData,
};
use nym_types::transaction::{
    RpcTransactionResponse, SendTxResult, TransactionDetails, TransactionExecuteResult,
};
use nym_types::vesting::{OriginalVestingResponse, PledgeData, VestingAccountInfo, VestingPeriod};
use nym_vesting_contract_common::Period;
use nym_wallet_types::admin::{
    TauriContractStateParams, TauriOperatingCostRange, TauriProfitMarginRange,
};
use nym_wallet_types::app::AppEnv;
use nym_wallet_types::app::AppVersion;
use nym_wallet_types::interval::Interval;
use nym_wallet_types::network::Network;
use nym_wallet_types::network_config::{Validator, ValidatorUrl, ValidatorUrls};
use std::path::Path;
use ts_rs::TS;
use walkdir::WalkDir;

macro_rules! do_export {
    ($a:ty) => {{
        match <$a>::export() {
            Ok(()) => {
                println!("✅ {}", <$a>::name());
            }
            Err(e) => {
                println!("❌ {} failed: {}", <$a>::name(), e);
            }
        }
    }};
}

fn main() {
    println!("Starting export of types using ts-rs...");
    println!();

    //
    // macro expands into `println!("Exporting {}...", Type::name()); Type::export();` with some error handling
    //

    // common/cosmwasm-smart-contracts/vesting-contract/src
    do_export!(Period);

    // common/cosmwasm-smart-contracts/mixnet-contract/src
    do_export!(IntervalRewardParams);
    do_export!(IntervalRewardingParamsUpdate);
    do_export!(MixNode);
    do_export!(MixNodeConfigUpdate);
    do_export!(RewardingParams);
    do_export!(RewardedSetNodeStatus);
    do_export!(UnbondedMixnode);
    do_export!(RewardEstimate);
    do_export!(ContractInterval);

    // common/types/src
    do_export!(Account);
    do_export!(AccountEntry);
    do_export!(AccountWithMnemonic);
    do_export!(Balance);
    do_export!(Delegation);
    do_export!(DelegationEvent);
    do_export!(DelegationEventKind);
    do_export!(DelegationResult);
    do_export!(DelegationsSummaryResponse);
    do_export!(DelegationWithEverything);
    do_export!(FeeDetails);
    // I'm explicitly using full(-ish) path as to indicate
    // those are not "proper" types to be used elsewhere
    do_export!(fees::ts_type_helpers::Fee);
    do_export!(fees::ts_type_helpers::CosmosFee);
    do_export!(fees::ts_type_helpers::Coin);
    do_export!(Gas);
    do_export!(GasInfo);
    do_export!(Gateway);
    do_export!(GatewayConfigUpdate);
    do_export!(GatewayBond);
    do_export!(CurrencyDenom);
    do_export!(DecCoin);
    do_export!(MixNodeBond);
    do_export!(MixNodeCostParams);
    do_export!(MixNodeDetails);
    do_export!(MixNodeRewarding);
    do_export!(OriginalVestingResponse);
    do_export!(PendingEpochEvent);
    do_export!(PendingEpochEventData);
    do_export!(PendingIntervalEvent);
    do_export!(PendingIntervalEventData);
    do_export!(PledgeData);
    do_export!(SendTxResult);
    do_export!(TransactionDetails);
    do_export!(SendTxResult);
    do_export!(TransactionExecuteResult);
    do_export!(RpcTransactionResponse);
    do_export!(VestingAccountInfo);
    do_export!(VestingPeriod);
    do_export!(WrappedDelegationEvent);

    // nym-api-requests
    do_export!(MixnodeCoreStatusResponse);
    do_export!(GatewayCoreStatusResponse);
    do_export!(InclusionProbabilityResponse);
    do_export!(MixnodeStatus);
    do_export!(MixnodeStatusResponse);
    do_export!(SelectionChance);
    do_export!(StakeSaturationResponse);
    do_export!(RewardEstimationResponse);

    // nym-wallet
    do_export!(AppEnv);
    do_export!(AppVersion);
    do_export!(Interval);
    do_export!(Network);
    do_export!(TauriContractStateParams);
    do_export!(TauriOperatingCostRange);
    do_export!(TauriProfitMarginRange);
    do_export!(Validator);
    do_export!(ValidatorUrl);
    do_export!(ValidatorUrls);

    let dst_base = Path::new("../../");

    println!();
    println!("Moving output files into place...");

    for file in WalkDir::new("./")
        .into_iter()
        .filter_map(|file| file.ok())
        .filter(|f| {
            let path = format!("{}", f.path().display());
            path != "./"
                && !path.starts_with("./src")
                && !path.starts_with("./target")
                && !path.starts_with("./Cargo.toml")
                && !path.starts_with("./.gitignore")
                && f.file_type().is_file()
        })
    {
        // construct the source and destination paths that can be used to replace the output file
        let src = file.path();
        let dst = dst_base.join(src);
        let dst_directory = dst.parent().expect("Could not get parent directory");

        if !dst_directory.exists() {
            if let Err(e) = std::fs::create_dir_all(dst_directory) {
                // show an error and move onto next file
                println!("❌ {}: {}", file.path().display(), e);
                continue;
            }
        }

        match std::fs::copy(src, dst.clone()) {
            Ok(_) => match std::fs::canonicalize(dst) {
                Ok(res) => {
                    println!("✅ {}  =>  {}", file.path().display(), res.display());
                }
                Err(e) => {
                    println!("❌ {}: {}", file.path().display(), e);
                }
            },
            Err(e) => {
                println!("❌ {}: {}", file.path().display(), e);
            }
        }
    }

    println!();
    println!("Done");
}
