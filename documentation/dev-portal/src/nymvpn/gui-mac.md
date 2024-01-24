# NymVPN alpha GUI: Guide for Mac OS

```admonish info
NymVPN is an experimental software and it's for [testing](./testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the workshop) so we use their results to improve the client, and submit the form [*NymVPN User research*]({{nym_vpn_form_url}}) with the testing results.
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### Installation

1. Create a directory `~/nym-vpn-latest`
```sh
mkdir -p "$HOME/nym-vpn-latest"
```
2. Open Github [releases page]({{nym_vpn_latest_binary_url}}) and download the binary for MacOS
3. Verify sha hash of your downloaded binary with the one listed on the [releases page]({{nym_vpn_latest_binary_url}}). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c
# Example:
echo "da4c0bf8e8b52658312d341fa3581954cfcb6efd516d9a448c76d042a454b5df  nym-vpn-desktop_0.0.3_macos_x86_64.zip" | shasum -a 256 -c
```
4. Extract files with `unzip` command or manually as you are used to
5. Move to the application directory and make executable
```sh
cd "macos/nym-vpn.app/Contents/MacOS"

chmod u+x nym-vpn
```
6. Move `nym-vpn` to your `~/nym-vpn-latest` directory
```sh
mv nym-vpn "$HOME/nym-vpn-latest"
```

### Configuration

7. Create the configuration file by opening a text editor and saving the lines below as `config.toml` in the same directory `~/nym-vpn-latest`
```toml
env_config_file = ".env"
entry_node_location = "DE" # two letters country code
# You can choose different entry by entering one of the following two letter country codes:
# DE, UK, FR, IE
```
8. Create testnet configuration file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `.env` in the same directory `~/nym-vpn-latest`
```sh
curl -L "https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env" -o "$HOME/nym-vpn-latest/.env"
```
## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

Open terminal in your `~/nym-vpn-latest` directory and run:
```sh
sudo ./nym-vpn
```

In case of errors check out the [troubleshooting](troubleshooting.html#installing-gui-on-macos-not-working) section.
