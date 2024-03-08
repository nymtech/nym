# NymVPN Command Line Interface (CLI)

```admonish info
Our alpha testing round is done with participants at live workshop events. This guide will not work for everyone, as the NymVPN source code is not yet publicly accessible. The alpha testing is done on Nym testnet Sandbox environment, this configuration is limited and will not work in the future.

**If you commit to test NymVPN alpha, please start with the [user research form]({{nym_vpn_form_url}}) where all the steps will be provided**. If you disagree with any of the conditions listed, please leave this page.
```

Follow the simple [automated script](#automated-script-for-cli-installation) below to install and run NymVPN CLI. If you prefer to do a manual setup follow the steps in the guide for [Linux](cli-linux.md) or [MacOS](cli-mac.md).

Visit NymVPN alpha latest [release page]({{nym_vpn_releases}}) to check sha sums or download the binaries directly.

## Automated Script for CLI Installation

We wrote a [script](https://gist.github.com/tommyv1987/87267ded27e1eb7651aa9cc745ddf4af) which does download of the CLI, sha256 verification, extraction, installation and configuration for Linux and MacOS users automatically following the steps below:

1. Open a terminal window in a directory where you want the script and NymVPN CLI binary be downloaded and run
```sh
curl -o execute-nym-vpn-cli-binary.sh -L https://gist.githubusercontent.com/tommyv1987/87267ded27e1eb7651aa9cc745ddf4af/raw/df78e30101c7357b57b311e06d5487e15d3335cb/execute-nym-vpn-cli-binary.sh && chmod u+x execute-nym-vpn-cli-binary.sh && sudo -E ./execute-nym-vpn-cli-binary.sh
```

2. Follow the prompts in the program

3. The script will automatically start the client. Make sure to **turn off any other VPNs** and follow the prompts:

* It prints a JSON view of existing Gateways and prompt you to:
    - *Make sure to use two different Gateways for entry and exit!*
    - `enter a gateway ID:` paste one of the values labeled with a key `"identityKey"` printed above (without `" "`)
    - `enter an exit address:` paste one of the values labeled with a key `"address"` printed above (without `" "`)
    - `do you want five hop or two hop?`: type `five` or `two`
    - `enable WireGuard? (yes/no):` if you chose yes, find your private key and wireguard IP [here](https://nymvpn.com/en/alpha)

To run `nym-vpn-cli` again, reconnect your wifi, move to the directory of your CLI binary `cd ~/nym-vpn-cli-dir` and follow the guide for [Linux](cli-linux.md#run-nymvpn) or [MacOS](cli-mac.md#run-nymvpn). If you find it too difficult, just run this script again - like in step \#3 above.

In case of errors check out the [troubleshooting](troubleshooting.md) section.
