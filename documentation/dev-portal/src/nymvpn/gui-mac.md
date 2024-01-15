# NymVPN alpha GUI: Guide for Mac OS

```admonish warning
NymVPN is an experimental software and it's for [testing](./nym-vpn-testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the event), and follow the steps listed in the form [*NymVPN User research*](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2).
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### Installation

1. Create a directory `~/nym-vpn-latest`
```sh
mkdir -p "$HOME/nym-vpn-latest"
```
2. Open Github [releases page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) to download the binary for Debian based Linux
3. Open terminal in the same directory and check the the `sha256sum` by running:
```sh
# aarch64
sha256sum ./nym-vpn-ui_0.0.2_macos_aarch64.zip

# x86_64
sha256sum ./nym-vpn-ui_0.0.2_macos_x86_64.zip
```
4. Compare the output with the sha256 hash shared on the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2)

5. Extract files with `unzip` command or manually as you are used to
6. Move to the application directory and make executable
```sh
cd "macos/nym-vpn.app/Contents/MacOS"

chmod u+x nym-vpn
```
7. Move `nym-vpn` to your `~/nym-vpn-latest` directory
```sh
mv nym-vpn "$HOME/nym-vpn-latest"
```

### Configuration

8. Create the configuration file by opening a text editor and saving the lines below as `config.toml` in the same directory `~/nym-vpn-latest`
```toml
env_config_file = ".env"
entry_node_location = "DE" # two letters country code
# You can choose different entry by entering one of the following two letter country codes:
# DE, UK, FR, IE
```
9. Create testnet configuration file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `.env` in the same directory `~/nym-vpn-latest`
```sh
curl -L "https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env" -o .env
```
## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

Open terminal in your `~/nym-vpn-latest` directory and run:
```sh
sudo ./nym-vpn
```

In case of errors check out the [troubleshooting](troubleshooting.html#installing-gui-on-macos-not-working) section.


