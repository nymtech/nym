pub(crate) mod args;

use args::{
    CONFIG_ENV_FILE, ENABLED_CREDENTIALS_MODE_ARG_NAME, ID, MIN_GATEWAY_RELIABILITY_ARG,
    MIN_MIXNODE_RELIABILITY_ARG, MIXNET_CONTRACT_ARG, MNEMONIC_ARG, MONITORING_ENABLED,
    NYXD_VALIDATOR_ARG, REWARDING_ENABLED, REWARDING_MONITOR_THRESHOLD_ARG, WRITE_CONFIG_ARG,
};
use clap::{crate_version, App, Arg, ArgMatches};

#[cfg(feature = "coconut")]
use args::{ANNOUNCE_ADDRESS, COCONUT_ENABLED};

pub fn parse_args() -> ArgMatches {
    let build_details = crate::long_version();
    let base_app = App::new("Nym API")
        .version(crate_version!())
        .long_version(&*build_details)
        .author("Nym")
        .arg(
            Arg::with_name(CONFIG_ENV_FILE)
                .help("Path pointing to an env file that configures the Nym API")
                .long(CONFIG_ENV_FILE)
                .short('c')
                .takes_value(true)
        )
        .arg(
            Arg::with_name(ID)
                .help("Id of the nym-api we want to run")
                .long(ID)
                .takes_value(true)
        )
        .arg(
            Arg::with_name(MONITORING_ENABLED)
                .help("specifies whether a network monitoring is enabled on this API")
                .long(MONITORING_ENABLED)
                .short('m')
        )
        .arg(
            Arg::with_name(REWARDING_ENABLED)
                .help("specifies whether a network rewarding is enabled on this API")
                .long(REWARDING_ENABLED)
                .short('r')
                .requires_all(&[MONITORING_ENABLED, MNEMONIC_ARG])
        )
        .arg(
            Arg::with_name(NYXD_VALIDATOR_ARG)
                .help("Endpoint to nyxd instance from which the monitor will grab nodes to test")
                .long(NYXD_VALIDATOR_ARG)
                .takes_value(true)
        )
        .arg(Arg::with_name(MIXNET_CONTRACT_ARG)
                 .long(MIXNET_CONTRACT_ARG)
                 .help("Address of the mixnet contract managing the network")
                 .takes_value(true),
        )
        .arg(Arg::with_name(MNEMONIC_ARG)
                 .long(MNEMONIC_ARG)
                 .help("Mnemonic of the network monitor used for rewarding operators")
                 .takes_value(true)
        )
        .arg(
            Arg::with_name(WRITE_CONFIG_ARG)
                .help("specifies whether a config file based on provided arguments should be saved to a file")
                .long(WRITE_CONFIG_ARG)
                .short('w')
        )
        .arg(
            Arg::with_name(REWARDING_MONITOR_THRESHOLD_ARG)
                .help("Specifies the minimum percentage of monitor test run data present in order to distribute rewards for given interval.")
                .takes_value(true)
                .long(REWARDING_MONITOR_THRESHOLD_ARG)
        )
        .arg(
            Arg::with_name(MIN_MIXNODE_RELIABILITY_ARG)
                .long(MIN_MIXNODE_RELIABILITY_ARG)
                .help("Mixnodes with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(MIN_GATEWAY_RELIABILITY_ARG)
                .long(MIN_GATEWAY_RELIABILITY_ARG)
                .help("Gateways with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .long(ENABLED_CREDENTIALS_MODE_ARG_NAME)
                .help("Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement")
        );

    #[cfg(feature = "coconut")]
    let base_app = base_app
        .arg(
            Arg::with_name(ANNOUNCE_ADDRESS)
                .help("Announced address where coconut clients will connect.")
                .long(ANNOUNCE_ADDRESS)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(COCONUT_ENABLED)
                .help("Flag to indicate whether coconut signer authority is enabled on this API")
                .requires_all(&[MNEMONIC_ARG, ANNOUNCE_ADDRESS])
                .long(COCONUT_ENABLED),
        );
    base_app.get_matches()
}
