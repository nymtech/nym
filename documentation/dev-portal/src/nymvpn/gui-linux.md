# NymVPN alpha GUI: Guide for GNU/Linux

```admonish warning
NymVPN is an experimental software and it's for [testing](testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the event), and follow the steps listed in the form [*NymVPN User research*](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2).
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### Installation

1. Open Github [releases page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) to download the binary for Debian based Linux
2. Open terminal in the same directory and check the the `sha256sum` by running:
```sh
sha256sum ./nym-vpn-ui_0.0.2_ubuntu-22.04_amd64.zip
```
2. Compare the output with the sha256 hash shared on the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2)
3. Extract files with `unzip` command or manually as you are used to
4. If you prefer to run `.AppImage` make executable by running:
```sh
chmod u+x ./appimage/nym-vpn_0.0.2_amd64.AppImage
# make sure your path to package is correct and the package name as well
```
5. If you prefer to use the `.deb` version for installation (Linux only), open terminal in the same directory and run:
```sh
cd deb

sudo dpkg -i ./nym-vpn_0.0.2_amd64.deb
# or
sudo apt-get install -f ./nym-vpn_0.0.2_amd64.deb

```

### Configuration

6. Create a NymVPN config directory called `nym-vpn` in your `~/.config`, either manually or by a command:
```sh
mkdir $HOME/.config/nym-vpn/
```
7. Create the network config by saving [this](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) as `sandbox.env` in the directory `~/.config/nym-vpn/` you just created by running:
```sh
curl -o $HOME/.config/nym-vpn/sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```

8. Create NymVPN main config file called `config.toml` in the same directory `~/.config/nym-vpn/` with this content:
```toml
# change <USER> to your username
env_config_file = "/home/<USER>/.config/nym-vpn/sandbox.env"
entry_node_location = "DE" # two letters country code
# You can choose different entry by entering one of the following two letter country codes:
# DE, UK, FR, IE
```

## Run NymVPN 

**For NymVPN to work, all other VPNs must be switched off!** At this alpha stage of NymVPN, the network connection (wifi) must be reconnected after or in between the testing rounds.

In case you used `.deb` package and installed the client, you may be able to have a NymVPN application icon in your app menu. However this may not work as the application needs root permission.

Open terminal and run:

```sh
# .AppImage must be run from the same directory as the binary
sudo -E ./nym-vpn_0.0.2_amd64.AppImage

# .deb installation shall be executable from anywhere as
sudo -E nym-vpn
```

In case of errors, see [troubleshooting section](troubleshooting.md).

