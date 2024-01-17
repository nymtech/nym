# NymVPN Command Line Interface (CLI)

```admonish info
Our alpha testing round is done with participants at live workshop events. This guide will not work for everyone, as the NymVPN source code is not yet publicly accessible. The alpha testing is done on Nym testnet Sandbox environment, this configuration is limited and will not work in the future.

**If you commit to test NymVPN alpha, please start with the [user research form]({{nym_vpn_form_url}}) where all the steps will be provided**. If you disagree with any of the conditions listed, please leave this page.
```

NymVPN CLI is a fundamental way to run the client for different purposes, currently it is a must for users who want to run the [testing scripts](testing.md).

Follow the simple [automated script](#automated-script-for-cli-installation) below to install and run NymVPN CLI. If you prefer to do a manual setup follow the steps in the guide for [Linux](cli-linux.md) or [MacOS](cli-mac.md).

## Automated Script for CLI Installation

We wrote a [script](https://gist.github.com/tommyv1987/87267ded27e1eb7651aa9cc745ddf4af/) which does download, sha256 verification, extraction, installation and configuration for Linux and MacOS users automatically following the steps below:

1. Download the script and save it as `nym-vpn-client-executor.sh`: 
```sh
curl -o nym-vpn-client-executor.sh -L https://gist.githubusercontent.com/tommyv1987/87267ded27e1eb7651aa9cc745ddf4af/raw/99cea8f4d80f2d002802ed1cbedba288bfca4488/execute-nym-vpn-cli-binary.sh
```
2. Make the script executable
```sh
chmod u+x nym-vpn-client-executor.sh
```
3. Run the script as root, turn of any VPN and run
```sh
sudo ./nym-vpn-client-executor.sh
```
4. Verify the `nym-vpn-cli` binary: When prompted to verify `sha256sum` paste your correct one from the [release page]({{nym_vpn_latest_binary_url}}) including the binary name (all as one input with a space in between), for example:
```sh
# Choose one according to the system you use, this is just an example
96623ccc69bc4cc0e4e3e18528b6dae6be69f645d0a592d926a3158ce2d0c269  nym-vpn-cli_0.1.0_macos_x86_64.zip
```
5. The script will automatically start the client. Follow the instructions:  

* It prints a JSON view of existing Gateways and prompt you to:
    - *Make sure to use two different Gateways for entry and exit!*
    - `enter a gateway ID:` paste one of the values labeled with a key `"identityKey"` printed above (without `" "`)
    - `enter an exit address:` paste one of the values labeled with a key `"address"` printed above (without `" "`)
    - `do you want five hop or two hop?`: type `five` or `two`
    - `enable WireGuard? (yes/no):` if you chose yes, find your private key and wireguard IP [here](https://nymvpn.com/en/alpha)

6. To run the `nym-vpn-cli` again, reconnect your wifi and follow the easy steps for [Linux](cli-linux.md#run-nymvpn) or [MacOS](cli-mac.md#run-nymvpn)

