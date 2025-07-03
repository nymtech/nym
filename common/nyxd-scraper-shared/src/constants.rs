// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use const_format::concatcp;

// TODO: make those configurable via 'NymNetworkDetails'

// BECH32_PREFIX defines the main SDK Bech32 prefix of an account's address
pub const BECH32_PREFIX: &str = "n";

// ACCOUNT_PREFIX is the prefix for account keys
pub const ACCOUNT_PREFIX: &str = "acc";
// VALIDATOR_PREFIX is the prefix for validator keys
pub const VALIDATOR_PREFIX: &str = "val";
// CONSENSUS_PREFIX is the prefix for consensus keys
pub const CONSENSUS_PREFIX: &str = "cons";
// PUBKEY_PREFIX is the prefix for public keys
pub const PUBKEY_PREFIX: &str = "pub";
// OPERATOR_PREFIX is the prefix for operator keys
pub const OPERATOR_PREFIX: &str = "oper";
// ADDRESS_PREFIX is the prefix for addresses
pub const ADDRESS_PREFIX: &str = "addr";

// BECH32_ACCOUNT_ADDRESS_PREFIX defines the Bech32 prefix of an account's address
pub const BECH32_ACCOUNT_ADDRESS_PREFIX: &str = BECH32_PREFIX;
// BECH32_ACCOUNT_PUBKEY_PREFIX defines the Bech32 prefix of an account's public key
pub const BECH32_ACCOUNT_PUBKEY_PREFIX: &str = concatcp!(BECH32_PREFIX, PUBKEY_PREFIX);
// BECH32_VALIDATOR_ADDRESS_PREFIX defines the Bech32 prefix of a validator's operator address
pub const BECH32_VALIDATOR_ADDRESS_PREFIX: &str =
    concatcp!(BECH32_PREFIX, VALIDATOR_PREFIX, OPERATOR_PREFIX);
// BECH32_VALIDATOR_PUBKEY_PREFIX defines the Bech32 prefix of a validator's operator public key
pub const BECH32_VALIDATOR_PUBKEY_PREFIX: &str = concatcp!(
    BECH32_PREFIX,
    VALIDATOR_PREFIX,
    OPERATOR_PREFIX,
    PUBKEY_PREFIX
);
// BECH32_CONSENSUS_ADDRESS_PREFIX defines the Bech32 prefix of a consensus node address
pub const BECH32_CONSENSUS_ADDRESS_PREFIX: &str =
    concatcp!(BECH32_PREFIX, VALIDATOR_PREFIX, CONSENSUS_PREFIX);
// BECH32_CONESNSUS_PUBKEY_PREFIX defines the Bech32 prefix of a consensus node public key
pub const BECH32_CONESNSUS_PUBKEY_PREFIX: &str = concatcp!(
    BECH32_PREFIX,
    VALIDATOR_PREFIX,
    CONSENSUS_PREFIX,
    PUBKEY_PREFIX
);
