# NymVPN Application (GUI)

```admonish info
Our alpha testing round is done with participants at live workshop events. This guide will not work for everyone, as the NymVPN source code is not yet publicly accessible. The alpha testing is done on Nym testnet Sandbox environment, this congiguration is limited and will not work in the future.

**If you commit to test NymVPN alpha, please start with the [user research form](https://opnform.com/forms/nymvpn-user-research-at-37c3-yccqko-2) where all the steps will be provided**. If you disagree with any of the conditions listed, please leave this page.
```

This is the alpha version of NymVPN application - the GUI. A demo of how the client will look like for majority of day-to-day users. For qualitative testing the [CLI](cli.md) is a necessity but to run the GUI holds the same importance as it provides the user with an experience of the actual app and the developers with a valuable feedback from the users. Below is an [automated script](#automated-script-for-gui-installation) for MacOS users (soon for Linux as well), if you prefer to do your own setup go to the page according to your operation system:

* [NymVPN GUI for GNU/Linux](gui-linux.md)
* [NymVPN GUI for MacOS](gui-mac.md)

## Automated Script for GUI Installation

We created a [script](https://gist.github.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19) which does download, extraction, installation and configuration for MacOS users automatically following the steps below:

1. To download the script, open a terminal in a directory where you want to download the script and run:
```sh
curl -o nym-vpn-client-executor.sh - L https://gist.githubusercontent.com/tommyv1987/7d210d4daa8f7abc61f9a696d0321f19/raw/4397365b4cf74594c7f99c1ef5d690b2f5b41192/nym-vpn-client-macos-executor.sh
```
2. Make the script executable
```sh
chmod u+x nym-vpn-client-macos-executor.sh
```
3. Run the script
```sh
./nym-vpn-client-macos-executor.sh
```
4. Verify the `nym-vpn` binary: When prompted to verify `sha256sum` paste one from the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) including the binary name (all as one input with a space in between), for example:
```sh
06c7c82f032f230187da1002a9a9a88242d3bbf6c5c09dc961a71df151d768d0  nym-vpn-ui_0.0.2_macos_x86_64.zip
```
5. The script will run the application and it will prompt you for a country code to exit, chose one of the offered options in the same format
6. To run the application again, follow the easy steps for [Linux](gui-linux.md#run-nymvpn) or [MacOS](gui-macos.md#run-nymvpn)

In case of errors check out the [troubleshooting](./nym-vpn-troubleshooting.html#installing-gui-on-macos-not-working) section.
