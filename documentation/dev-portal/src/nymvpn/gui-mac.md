# NymVPN alpha - Desktop: Guide for Mac OS

```admonish info
NymVPN is an experimental software and it's for [testing](./testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the workshop) so we use their results to improve the client, and submit the form [*NymVPN User research*]({{nym_vpn_form_url}}) with the testing results.
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### Installation

<!-- Seems redundant
1. Create a directory `~/nym-vpn-latest`
```sh
mkdir -p "$HOME/nym-vpn-latest"
```
-->

1. Open Github [releases page]({{nym_vpn_releases}}) and download the binary for your version of MacOS

2. Recommended (skip this point if you don't want to verify): Verify sha hash of your downloaded binary with the one listed on the [releases page]({{nym_vpn_releases}}). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c

# choose a correct one according to your binary, this is just an example
# echo "da4c0bf8e8b52658312d341fa3581954cfcb6efd516d9a448c76d042a454b5df  nym-vpn-desktop_0.0.3_macos_x86_64.zip" | shasum -a 256 -c
```

3. Extract the downloaded file manually or by a command:
```sh
tar -xvf <BINARY>.tar.gz
# for example
# tar -xvf nym-vpn-desktop_0.0.4_macos_aarch64.tar.gz
```
<!-- seems redundant
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
-->
4. Mount the `.dmg` image you extracted by double clicking on it and move it (drag it) to your `/Application` folder


NymVPN alpha version runs over Nym testnet (called sandbox), a little extra configuration is needed for the application to work.

### Configuration

To test NymVPN alpha we must create two configuration files: an environment config file `sandbox.env` and `config.toml` file pointing the application to run over the testnet environment.

5. Create testnet configuration file: Open a text editor, copy-paste [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) and save it as `sandbox.env` in `/Applications/nym-vpn.app/Contents/MacOS/`. Alternatively use this command:
```sh
curl -L "https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env" -o "/Applications/nym-vpn.app/Contents/MacOS/sandbox.env"
```

6. Create application configuration file: Open a text editor, copy-paste the line below and save as `config.toml` in the same directory `/Applications/nym-vpn.app/Contents/MacOS/`
```toml
env_config_file = "sandbox.env"
```
Alternatively do it by using this command:
```sh
echo "env_config_file = sandbox.env" > /Applications/nym-vpn.app/Contents/MacOS/config.toml
```
## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

Run:
```sh
sudo /Applications/nym-vpn.app/Contents/MacOS/nym-vpn

# If it didn't start try to run with -E flag
sudo -E /Applications/nym-vpn.app/Contents/MacOS/nym-vpn
```

In case of errors check out the  [troubleshooting](troubleshooting.md#running-gui-failed-due-to-toml-parse-error) section.
