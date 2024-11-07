# NymVPN - Desktop (GUI)

```admonish info
Our alpha testing round is done with participants at live workshop events and the application in this stage may not work for everyone.

**If you commit to test NymVPN alpha, please start with the [user research form]({{nym_vpn_form_url}}) where all the steps will be provided**. If you disagree with any of the conditions listed, please leave this page.
```

This is a desktop (GUI) version of NymVPN client. A demo of how the application will look like for majority of day-to-day users.

Follow the simple [automated script](#automated-script-for-gui-installation) below to install and run NymVPN GUI. If the script didn't work for your distribution or you prefer to do a manual setup follow the steps in the guide for [Linux](gui-linux.md) or [MacOS](gui-mac.md) .

Visit NymVPN alpha latest [release page](https://github.com/nymtech/nym-vpn-client) to check sha sums or download the binaries directly.

## Linux AppImage Automated Installation Method

The latest releases contain `appimage.sh` script. This method makes the installation simple for Linux users who want to run NymVPN from AppImmage. Executing the command below will download the binary to `~/.local/bin` and verify the checksum:
```sh
curl -fsSL https://github.com/nymtech/nym-vpn-client/releases/download/nym-vpn-desktop-v<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->/appimage.sh | bash
```

Run with the command:
```sh
sudo -E ~/.local/bin/nym-vpn.appimage
```

## Automated Script for GUI Installation (Linux and Mac)

We wrote a [script](https://gist.github.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19) which does download of dependencies and the application, sha256 verification, extraction, installation and configuration for Linux and MacOS users automatically. Turn off all VPNs and follow the steps below.

1. Open a terminal window in a directory where you want the script to be downloaded and run
```sh
curl -o nym-vpn-desktop-install-run.sh -L https://gist.githubusercontent.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19/raw/939ac8d0afed69f43739b9cf2e5728454ea2c437/nym-vpn-client-install-run.sh && chmod u+x nym-vpn-desktop-install-run.sh && sudo -E ./nym-vpn-desktop-install-run.sh
```

2. Follow the prompts in the program

To start the application again, reconnect your wifi and run
```sh
# Linux .AppImage
sudo -E ~/nym-vpn-latest/nym-vpn-desktop_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_ubuntu-22.04_x86_64/nym-vpn_<!-- cmdrun scripts/nym_vpn_desktop_version.sh -->_amd64.AppImage

# Linux .deb
sudo -E nym-vpn

# MacOS
sudo -E $HOME/nym-vpn-latest/nym-vpn
```

In case of errors check out the [troubleshooting](troubleshooting.md#running-gui-failed-due-to-toml-parse-error) section.
