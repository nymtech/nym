// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) mod vars {
    pub(crate) const ZULIP_BOT_EMAIL_ARG: &str = "ZULIP_BOT_EMAIL";
    pub(crate) const ZULIP_BOT_API_KEY_ARG: &str = "ZULIP_BOT_API_KEY";
    pub(crate) const ZULIP_SERVER_URL_ARG: &str = "ZULIP_SERVER_URL";
    pub(crate) const ZULIP_NOTIFICATION_CHANNEL_ID_ARG: &str = "ZULIP_NOTIFICATION_CHANNEL_ID";
    pub(crate) const ZULIP_NOTIFICATION_CHANNEL_TOPIC_ARG: &str =
        "ZULIP_NOTIFICATION_CHANNEL_TOPIC";

    pub(crate) const SIGNERS_MONITOR_CHECK_INTERVAL_ARG: &str = "SIGNERS_MONITOR_CHECK_INTERVAL";

    pub(crate) const KNOWN_NETWORK_NAME_ARG: &str = "KNOWN_NETWORK_NAME";
    pub(crate) const NYXD_CLIENT_CONFIG_ENV_FILE_ARG: &str = "NYXD_CLIENT_CONFIG_ENV_FILE";
    pub(crate) const NYXD_RPC_ENDPOINT_ARG: &str = "NYXD_RPC_ENDPOINT";
    pub(crate) const NYXD_DKG_CONTRACT_ADDRESS_ARG: &str = "NYXD_DKG_CONTRACT_ADDRESS";
}
