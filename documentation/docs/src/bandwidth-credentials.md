# Bandwidth Credentials

You can now try using Nym Bandwidth Credentials in our [Sandbox testnet environment](https://sandbox-explorer.nymtech.net).

Create a `sandbox.env` file with the following details: 

```
CONFIGURED=true

NETWORK_NAME=sandbox

RUST_LOG=info
RUST_BACKTRACE=1

BECH32_PREFIX=nymt
MIX_DENOM=unymt
MIX_DENOM_DISPLAY=nymt
STAKE_DENOM=unyxt
STAKE_DENOM_DISPLAY=nyxt
DENOMS_EXPONENT=6

REWARDING_VALIDATOR_ADDRESS="nymt1mxuweurc066kprnngtm8zmvam7m2nw26yatpmv"
MIXNET_CONTRACT_ADDRESS="nymt1dlsvvgey26ernlj0sq2afjluh3qd4ap0k9eerekfkw5algqrwqksaf2qf7"
VESTING_CONTRACT_ADDRESS="nymt19g9xuqrvz2frv905v3fc7puryfypluhg383q9zwsmedrlqekfgys62ykm4"
MULTISIG_CONTRACT_ADDRESS="nymt142dkm8xe9f0ytyarp7ww4kvclva65705jphxsk9exn3nqdsm8jkqnp06ac"
COCONUT_BANDWIDTH_CONTRACT_ADDRESS="nymt1ty0frysegskh6ndm3v96z5xdq66qzcu0aw7xcxlgp54jg0mjwlgqplc6v0"
COCONUT_DKG_CONTRACT_ADDRESS="nymt1gwk6muhmzeuxje7df7rjvqwl2vex0kj4t2hwuzmyx5k62kfusu5qk4k5z4"
GROUP_CONTRACT_ADDRESS="nymt14ry36mwauycz08v8ndcujghxz4hmua5epxcn0mamlr3suqe0l2qsqx5ya2"

STATISTICS_SERVICE_DOMAIN_ADDRESS="http://0.0.0.0"
NYXD="https://rpc.sandbox.nymtech.net"
NYM_API="https://sandbox-validator1-api.nymtech.net/api"
```

Create an account on Sandbox using the nym-cli: 

```./nym-cli --config-env-file <path-to>sandbox.env account create```

You will need `nymt` funds sent to this account. Get in touch via Nym [Telegram](https://t.me/nymchan) or [Discord](https://discord.gg/FaTJb8q8) and we can send them to you. 

Next, you init the nym-client with the enabled credentials mode set to true:  

```./nym-client --config-env-file <path-to>sandbox.env init --id <ID> --enabled-credentials-mode true```

Using the new credentials binary, purchase some credentials for the client. The recovery directory is a directory where the credentials will be temporarily stored in case the request fails.  

```./credential --config-env-file <path-to>sandbox.env run --client-home-directory <path-to-the-client-config> --nyxd-url https://rpc.sandbox.nymtech.net --mnemonic "<mnemonic of the account created above>" --amount 50 --recovery-dir <a-path> ```

You can redeem this now by running the nym-client, in enabled credentials mode:

```./nym-client --config-env-file <path-to>sandbox.env run --id <ID> --enabled-credentials-mode true```

Run the network requester which can be downloaded [here](https://github.com/nymtech/nym/releases/download/nym-binaries-v1.1.9/nym-network-requester)

 `./nym-network-requester run`

> You need to run this version for now, as the `nym-client` functionality was recently integrated into the `network-requester` binary but for the moment cannot support coconut credentials natively.

Now time to init the socks5 client: 

`./nym-socks5-client --config-env-file <path-to>sandbox.env init --id <ID> --provider <insert provider address which was returned when init-ing the nym-client> --enabled-credentials-mode true`

Purchase credentials for this now too: 

`./credential --config-env-file <path-to>sandbox.env run --client-home-directory <path-to-socks5-config> --nyxd-url https://rpc.sandbox.nymtech.net --mnemonic "<any valid sandbox mnemonic>" --amount 100 --recovery-dir <a-path>`

Run the socks5 client:

`./nym-socks5-client --config-env-file <path-to>sandbox.env run --id <ID> --enabled-credentials-mode true`

NOTE 

You can check to see if credentials have been correctly purchased by installing sqlite, and proceeding to do the following:

```
sqlite3 ~/.nym/socks5-clients/<ID>/data/credentials_database.db  
select * from coconut_credentials;
```

Keep in mind 1GB = 1NYM 
