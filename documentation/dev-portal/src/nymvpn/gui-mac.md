# NymVPN alpha - Desktop: Guide for Mac OS

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

2. Open Github [releases page]({{nym_vpn_latest_binary_url}}) and download the binary for your version of MacOS

3. Recommended (skip to next point if you don't want to verify): Verify sha hash of your downloaded binary with the one listed on the [releases page]({{nym_vpn_latest_binary_url}}). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c

# choose a correct one according to your binary, this is just an example
# echo "da4c0bf8e8b52658312d341fa3581954cfcb6efd516d9a448c76d042a454b5df  nym-vpn-desktop_0.0.3_macos_x86_64.zip" | shasum -a 256 -c
```

4. Extract files:
```sh
tar -xvf <BINARY>
# for example
# tar -xvf nym-vpn-desktop_0.0.4_macos_aarch64.tar.gz
```

5. Move to the application content directory:
```sh
cd "macos/nym-vpn.app/Contents/MacOS"

# if it didn't work, try
cd "/Applications/nym-vpn.app/Contents/MacOS/"
```

6. Make executable
```sh
chmod u+x nym-vpn
```

7. Move `nym-vpn` to your `~/nym-vpn-latest` directory
```sh
mv nym-vpn "$HOME/nym-vpn-latest"
```

Here you are basically done with the installation. NymVPN alpha version runs over Nym testnet (called sandbox), we need to do a little configuration:

### Configuration

8. Create the application configuration file by opening a text editor and saving the lines below as `config.toml` in the same directory `~/nym-vpn-latest`
```toml
env_config_file = ".env"
```
Alternatively do it via a simple command:
```sh
echo "env_config_file = .env" > "$HOME/nym-vpn-latest/config.toml"
```
9. Create testnet configuration file by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `.env` in the same directory `~/nym-vpn-latest`
```sh
curl -L "https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env" -o "$HOME/nym-vpn-latest/.env"
```
## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

Run:
```sh
sudo -E $HOME/nym-vpn-latest/nym-vpn
```

In case of errors check out the [troubleshooting](troubleshooting.html#installing-gui-on-macos-not-working) section.
