# NymVPN Application (GUI)

```admonish info
Our alpha testing round is done with participants at live workshop events. This guide will not work for everyone, as the NymVPN source code is not yet publicly accessible. The alpha testing is done on Nym testnet Sandbox environment, this configuration is limited and will not work in the future.

**If you commit to test NymVPN alpha, please start with the [user research form]({{nym_vpn_form_url}}) where all the steps will be provided**. If you disagree with any of the conditions listed, please leave this page.
```

This is the alpha version of NymVPN application - the GUI. A demo of how the client will look like for majority of day-to-day users. For qualitative testing the [CLI](cli.md) is a necessity but to run the GUI holds the same importance as it provides the user with an experience of the actual app and the developers with a valuable feedback from the users. 

Follow the simple [automated script](#automated-script-for-gui-installation) below to install and run NymVPN GUI. If you prefer to do a manual setup follow the steps in the guide for [Linux](gui-linux.md) or [MacOS](gui-mac.md).

## Automated Script for GUI Installation

We wrote a [script](https://gist.github.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19) which does download of dependencies and the application, sha256 verification, extraction, installation and configuration for Linux and MacOS users automatically following the steps below:

1. To download the script, open a terminal in a directory where you want to download the script and run:
```sh
curl -o nym-vpn-client-installer.sh -L https://gist.githubusercontent.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19/raw/181968941ce268a3937e82239ddfd293dd96bb60/nym-vpn-client-installer.sh
```
2. Make the script executable
```sh
chmod u+x nym-vpn-client-installer.sh
```
3. Run the script as root, turn off any VPN and run
```sh
sudo ./nym-vpn-client-installer.sh
```
4. Verify the `nym-vpn` binary: When prompted to verify `sha256sum` paste your correct one from the [release page]({{nym_vpn_latest_binary_url}}) including the binary name (all as one input with a space in between), for example:
```sh
# Choose one according to the system you use, this is just an example
06c7c82f032f230187da1002a9a9a88242d3bbf6c5c09dc961a71df151d768d0  nym-vpn-ui_0.0.2_macos_x86_64.zip
```
5. The script will run the application and it will prompt you for a country code to exit, chose one of the offered options in the same format as listed

6. To start the application again, reconnect your wifi and run
```sh
# Linux
sudo -E ~/nym-vpn-latest/nym-vpn_0.0.2_amd64.AppImage

# MacOS
sudo $nym_vpn_dir/nym-vpn
```

In case of errors check out the [troubleshooting](troubleshooting.md#installing-gui-on-macos-not-working) section.
