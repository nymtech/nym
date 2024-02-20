# NymVPN alpha GUI: Guide for GNU/Linux

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/908221306?h=404b2bbdc8" style="position:absolute;top:0;left:0;width:100%;height:100%;" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" allowfullscreen></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

```admonish info
NymVPN is an experimental software and it's for [testing](./testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the workshop) so we use their results to improve the client, and submit the form [*NymVPN User research*]({{nym_vpn_form_url}}) with the testing results.
```

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

### Installation

1. Open Github [releases page]({{nym_vpn_latest_binary_url}}) and download the binary for Debian based Linux
2. Verify sha hash of your downloaded binary with the one listed on the [releases page]({{nym_vpn_latest_binary_url}}). You can use a simple `shasum` command and compare strings (ie with Python) or run in the same directory the following command, exchanging `<SHA_STRING>` with the one of your binary, like in the example:
```sh
echo "<SHA_STRING>" | shasum -a 256 -c

# choose a correct one according to your binary, this is just an example
echo "a5f91f20d587975e30b6a75d3a9e195234cf1269eac278139a5b9c39b039e807  nym-vpn-desktop_0.0.3_ubuntu-22.04_x86_64.zip" | shasum -a 256 -c
```
3. Extract files with `unzip` command or manually as you are used to
4. If you prefer to run `.AppImage` make executable by running:
```sh
chmod u+x ./appimage/nym-vpn_0.0.2_amd64.AppImage
```
5. If you prefer to use the `.deb` version for installation (works on Debian based Linux only), open terminal in the same directory and run:
```sh
cd deb

sudo dpkg -i ./nym-vpn_0.0.3_amd64.deb
# or
sudo apt-get install -f ./nym-vpn_0.0.3_amd64.deb
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
