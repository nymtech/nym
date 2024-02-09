# NymVPN - Desktop (GUI)

```admonish info
Our alpha testing round is done with participants at live workshop events. This guide will not work for everyone, as the NymVPN source code is not yet publicly accessible. The alpha testing is done on Nym testnet Sandbox environment, this configuration is limited and will not work in the future.

**If you commit to test NymVPN alpha, please start with the [user research form]({{nym_vpn_form_url}}) where all the steps will be provided**. If you disagree with any of the conditions listed, please leave this page.
```

This is the alpha version of NymVPN desktop application (GUI). A demo of how the client will look like for majority of day-to-day users. For qualitative testing the [CLI](cli.md) is a necessity but to run the GUI holds the same importance as it provides the user with an experience of the actual app and the developers with a valuable feedback from the users.

Follow the simple [automated script](#automated-script-for-gui-installation) below to install and run NymVPN GUI. If you prefer to do a manual setup follow the steps in the guide for [Linux](gui-linux.md) or [MacOS](gui-mac.md).

Visit NymVPN alpha latest [release page]({{nym_vpn_latest_binary_url}}) to check sha sums or download the binaries directly.

## Automated Script for GUI Installation

We wrote a [script](https://gist.github.com/serinko/e0a9f7ff3d79e974ec6f6783caa1137e) which does download of dependencies and the application, sha256 verification, extraction, installation and configuration for Linux and MacOS users automatically following the steps below:

1. Open a terminal window in a directory where you want the script and NymVPN binary be downloaded and run
```sh
curl -o nym-vpn-desktop-install-run.sh -L https://gist.githubusercontent.com/serinko/e0a9f7ff3d79e974ec6f6783caa1137e/raw/064c0d42af6a187b4f62998a0c2e6a84c319eeef/nym-vpn-desktop-install-run.sh
```

2. Make the script executable
```sh
chmod u+x nym-vpn-desktop-install-run.sh
```

3. Start the script, turn off any VPN and run
```sh
sudo -E ./nym-vpn-desktop-install-run.sh
```

4. Follow the prompts in the program

To start the application again, reconnect your wifi and run
```sh
# Linux
sudo -E ~/nym-vpn-latest/nym-vpn_0.0.3_amd64.AppImage

# MacOS
sudo -E $HOME/nym-vpn-latest/nym-vpn
```

In case of errors check out the [troubleshooting](troubleshooting.md#installing-gui-on-macos-not-working) section.
