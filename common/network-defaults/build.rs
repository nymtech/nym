use regex::Regex;
use std::{collections::HashMap, fs};

const TARGET_ENV_FILE: &str = "../../envs/mainnet.env";

/// Sync variable values defined in code with .env file
fn main() {
    let file = include_str!("src/mainnet.rs");

    let variables_to_track = [
        "NETWORK_NAME",
        "BECH32_PREFIX",
        "MIXNET_CONTRACT_ADDRESS",
        "VESTING_CONTRACT_ADDRESS",
        "GROUP_CONTRACT_ADDRESS",
        "ECASH_CONTRACT_ADDRESS",
        "MULTISIG_CONTRACT_ADDRESS",
        "COCONUT_DKG_CONTRACT_ADDRESS",
        "REWARDING_VALIDATOR_ADDRESS",
        "NYM_API",
        "NYXD_WS",
        "EXPLORER_API",
        "NYM_VPN_API",
    ];

    let mut replace_with = HashMap::new();

    for var in variables_to_track {
        // if script fails, debug with `cargo check -vv``
        println!("Looking for {}", var);

        // read pattern that looks like:
        // <var>: &str = "<whatever is between quotes>"
        let pattern = format!(r#"{}: &str\s*=\s*"([^"]*)""#, regex::escape(var));

        let re = Regex::new(&pattern).unwrap();
        let value = re
            .captures(file)
            .and_then(|caps| caps.get(1).map(|match_| match_.as_str().to_string()))
            .expect("Couldn't find var in source file");
        println!("Storing {}={}", var, value);
        replace_with.insert(var, value);
    }

    let mut contents = fs::read_to_string(TARGET_ENV_FILE).unwrap();

    for (var, value) in replace_with {
        // match a pattern that looks like:
        // <var> = "<value>"
        // where `<var>` is a variable name inserted into search pattern
        let pattern = format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(var));

        // replace matched pattern with
        // <var>="<value>"
        let re = Regex::new(&pattern).unwrap();
        contents = re
            .replace(&contents, |_: &regex::Captures| {
                format!(r#"{}="{}""#, var, value)
            })
            .to_string();
    }

    fs::write(TARGET_ENV_FILE, contents).unwrap();
}
