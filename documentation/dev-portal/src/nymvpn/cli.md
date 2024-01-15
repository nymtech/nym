# NymVPN Command Line Interface (CLI)

NymVPN CLI is a fundamental way to run the client for different purposes, currently it is a must for users who want to run the [testing scripts](testing.md).

Below is a way to setup NymVPN CLI using an [automated script](#automated-script-for-cli-installation), if you prefer to do your own setup go to the page according to your operation system:

* [NymVPN CLI for GNU/Linux](cli-linux.md)
* [NymVPN CLI for MacOS](cli-mac.md)

## Automated Script for CLI Installation

Open a terminal and follow these steps:

1. Download the script and save it as `nym-vpn-client-executor.sh`: 
```sh
curl -o nym-vpn-client-executor.sh -L https://gist.githubusercontent.com/tommyv1987/87267ded27e1eb7651aa9cc745ddf4af/raw/99cea8f4d80f2d002802ed1cbedba288bfca4488/execute-nym-vpn-cli-binary.sh
```
2. Make the script executable
```sh
chmod u+x nym-vpn-client-executor.sh
```
3. Run the script
```sh
./nym-vpn-client-executor.sh
```
4. Verify the `nym-vpn-cli` binary: When prompted to verify `sha256sum` paste one from the [release page](https://github.com/nymtech/nym/releases/tag/nym-vpn-alpha-0.0.2) including the binary name (all as one input with a space in between), for example:
```sh
96623ccc69bc4cc0e4e3e18528b6dae6be69f645d0a592d926a3158ce2d0c269  nym-vpn-cli_0.1.0_macos_x86_64.zip
```

5. The script will automatically start the client. Follow the instructions:  

* It prints a JSON view of existing Gateways and prompt you to:
    - ***(Make sure to use two different Gateways for entry and exit!)***
    - `enter a gateway ID:` paste one of the values labeled with a key `"identityKey"` printed above (without `" "`)
    - `enter an exit address:` paste one of the values labeled with a key `"address"` printed above (without `" "`)
    - `do you want five hop or two hop?`: type `five` or `two`
    
    <!-- enter wireguard ID -->

6. To run the `nym-vpn-cli` again, follow the easy steps for [Linux](cli-linux.md#run-nymvpn) or [MacOS](cli-macos.md#run-nymvpn).
