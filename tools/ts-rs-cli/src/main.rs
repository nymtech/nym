use std::path::Path;
use ts_rs::TS;
use walkdir::WalkDir;

use mixnet_contract_common::mixnode::RewardedSetNodeStatus;
use nym_types::account::{Account, AccountEntry, AccountWithMnemonic, Balance};
use nym_types::currency::{CurrencyDenom, DecCoin, MajorAmountString, MajorCurrencyAmount};
use nym_types::delegation::{
    Delegation, DelegationEvent, DelegationEventKind, DelegationRecord, DelegationResult,
    DelegationWithEverything, DelegationsSummaryResponse, PendingUndelegate,
};
use nym_types::fees::FeeDetails;
use nym_types::gas::{Gas, GasInfo};
use nym_types::gateway::{Gateway, GatewayBond};
use nym_types::mixnode::{MixNode, MixNodeBond};
use nym_types::transaction::{
    RpcTransactionResponse, SendTxResult, TransactionDetails, TransactionExecuteResult,
};
use nym_types::vesting::{OriginalVestingResponse, PledgeData, VestingAccountInfo, VestingPeriod};
use nym_wallet_types::admin::TauriContractStateParams;
use nym_wallet_types::app::AppEnv;
use nym_wallet_types::epoch::Epoch;
use nym_wallet_types::network::Network;
use nym_wallet_types::network_config::{Validator, ValidatorUrl, ValidatorUrls};
use validator_api_requests::models::{
    CoreNodeStatusResponse, InclusionProbabilityResponse, MixnodeStatus, MixnodeStatusResponse,
    SelectionChance, StakeSaturationResponse,
};
use vesting_contract_common::Period;

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

    // common/cosmwasm-smart-contracts/mixnet-contract/src
    do_export!(RewardedSetNodeStatus);
    // common/cosmwasm-smart-contracts/vesting-contract/src
    do_export!(Period);

    // common/types/src
    do_export!(Account);
    do_export!(AccountEntry);
    do_export!(AccountWithMnemonic);
    do_export!(Balance);
    do_export!(CurrencyDenom);
    do_export!(Delegation);
    do_export!(DelegationEvent);
    do_export!(DelegationEventKind);
    do_export!(DelegationRecord);
    do_export!(DelegationResult);
    do_export!(DelegationsSummaryResponse);
    do_export!(DelegationWithEverything);
    do_export!(FeeDetails);
    do_export!(Gas);
    do_export!(GasInfo);
    do_export!(Gateway);
    do_export!(GatewayBond);
    do_export!(MajorAmountString);
    do_export!(MajorCurrencyAmount);
    do_export!(DecCoin);
    do_export!(MixNode);
    do_export!(MixNodeBond);
    do_export!(OriginalVestingResponse);
    do_export!(PendingUndelegate);
    do_export!(PledgeData);
    do_export!(SendTxResult);
    do_export!(TransactionDetails);
    do_export!(SendTxResult);
    do_export!(TransactionExecuteResult);
    do_export!(RpcTransactionResponse);
    do_export!(VestingAccountInfo);
    do_export!(VestingPeriod);

    // validator-api-requests
    do_export!(CoreNodeStatusResponse);
    do_export!(InclusionProbabilityResponse);
    do_export!(MixnodeStatus);
    do_export!(MixnodeStatusResponse);
    do_export!(SelectionChance);
    do_export!(StakeSaturationResponse);

    // nym-wallet
    do_export!(AppEnv);
    do_export!(Epoch);
    do_export!(Network);
    do_export!(TauriContractStateParams);
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
