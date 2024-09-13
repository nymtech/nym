# NymVPN alpha - Desktop: Guide for GNU/Linux

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/908221306?h=404b2bbdc8" style="position:absolute;top:0;left:0;width:100%;height:100%;" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" allowfullscreen></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

```admonish info
NymVPN is an experimental software and it's for testing purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the workshop) so we use their results to improve the client, and submit the form [*NymVPN User research*]({{nym_vpn_form_url}}) with the testing results.
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### Installation

1. Open Github [releases page]({{nym_vpn_releases}}) and download the binary for Debian based Linux

2. (Optional: if you don't want to check shasum, skip this point) Verify sha hash of your downloaded binary with the one listed on the [releases page]({{nym_vpn_releases}}). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c

# choose a correct one according to your binary, this is just an example
# echo "a5f91f20d587975e30b6a75d3a9e195234cf1269eac278139a5b9c39b039e807  nym-vpn-desktop_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_ubuntu-22.04_x86_64.tar.gz" | shasum -a 256 -c
```

3. Extract files:
```sh
tar -xvf <BINARY>.tar.gz
# for example
# tar -xvf nym-vpn-desktop_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_ubuntu-22.04_x86_64.tar.gz
```

4. If you prefer to run `.AppImage` make executable by running:
```sh
# make sure you cd into the right sub-directory after extraction
chmod u+x ./nym-vpn_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_amd64.AppImage
```

5. If you prefer to use the `.deb` version for installation (works on Debian based Linux only), open terminal in the same directory and run:
```sh
# make sure you cd into the right sub-directory after extraction
sudo dpkg -i ./nym-vpn_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_amd64.deb
# or
sudo apt-get install -f ./nym-vpn_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_amd64.deb
```

<!--
NymVPN alpha version runs over Nym testnet (called sandbox), a little extra configuration is needed for the application to work.

### Configuration

To test NymVPN alpha we must create two configuration files: an environment config file `sandbox.env` and `config.toml` file pointing the application to run over the testnet environment.

6. Create a NymVPN config directory called `nym-vpn` in your `~/.config`, either manually or by a command:
```sh
mkdir $HOME/.config/nym-vpn/
```
7. Create the network testnet config: copy-paste [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) and save as `sandbox.env` in the directory `~/.config/nym-vpn/` you just created. Aternatively do it by runnin a command
```sh
curl -o $HOME/.config/nym-vpn/sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```

8. Create NymVPN main config file: copy-paste the line below and save it as `config.toml` in the same directory `~/.config/nym-vpn/`:
```toml
# change <USER> to your username
env_config_file = "/home/<USER>/.config/nym-vpn/sandbox.env"
```
-->

## Run NymVPN

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

In case you used `.deb` package and installed the client, you may be able to have a NymVPN application icon in your app menu. However this may not work as the application needs root permission.

Open terminal and run:

```sh
# .AppImage must be run from the same directory as the binary
sudo -E ./nym-vpn_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_amd64.AppImage

# .deb installation shall be executable from anywhere as
sudo -E nym-vpn
```

In case of errors, see [troubleshooting section](troubleshooting.md).
