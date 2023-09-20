# HCPP 2023 - Securing the Lunarpunks Workshop

The workshop will introduce ***why*** and ***how to use Nym platform as a network protection*** layer when using some of our favorite privacy applications. This page serves as an accessible guide alongside the talk and it includes all the steps, per-requizities and dependencies needed. Preferably the users interested in this setup start downloading and building the tools before the workshop or in the beginning of it so the limited time can be used for questions and addressing problems. This guide will stay online for another week after the event just in case people were not finished and want to catch up later.  

This page is a *how to guide* so it contains the setup steps only, to see the entire presentation please come to XXX at YYY.

## Preparation

During this workshop we will introduce NymConnect and Socks5 client. The difference between them is that the Socks5 client does everything Nymconnect does, but it has more optionality as it's run in a commandline. NymConnect is a one-button GUI application that wraps around the `nym-socks5-client` for proxying application traffic through the Mixnet.  

We will learn how to run over Nym Mixnet the following applications: Electrum Bitcoin wallet, Monero wallet (desktop and CLI), Matrix (Element app) and ircd chat. For those who want to run ircd through the Mixnet, `nym-socks5-client` client is a must. For all other applications you can choose if you settle with our slick app NymConnect which does all the job in the background or you prefer Socks5 client.

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

## NymConnect Installation

NymConnect for everyone who does not want to install and run `nym-socks5-client`. NymConnect is plug and play - fast and easy to download and run. It connects automatically to Electrum Bitcoin wallet, Monero wallet (desktop and CLI) and Matrix (Element app) after we set them up.

1. [Download](https://nymtech.net/download/nymconnect) NymConnect
2. On Linux and Mac, make executable by opening terminal in the same directory and run:
```sh
chmod +x ./nym-connect_<VERSION>.AppImage
``` 
3. Start the application
4. Click on `Connect` button to initialize the connection with the Mixnet
5. Anytime later you'll need to setup Host and Port in your applications, click on `IP` and `Port` to copy the values to clipboard
6. In case you have problems such as `Gateway Issues`, try to reconnect or restart the application

## Building Nym Platform

If you prefer to run to run `nym-socks5-client` the possibility is to download the pre-build binary or build the entire platform. To run ircd through the mixnet `nym-socks5-client` and `nym-network-requester` are mandatory. Before you start with donwload and installation, make sure you are on the same machine from which you connect to ircd.

If you prefer to run to run `nym-socks5-client` the possibility is to download the pre-build binary or build the entire platform. To run ircd through the mixnet `nym-socks5-client` and `nym-network-requester` are mandatory. Before you start with download and installation, make sure you are on the same machine from which you connect to ircd.

We recommend to clone and build the entire platform instead of individual binaries as it offers an easier update and more options down the road, however it takes a basic commandline knowledge and longer time. The [Nym platform](https://github.com/nymtech/nym) is written in Rust. For that to work we will need a few pre-requisities. If you prefer to download individual pre-build binaries, skip this part and go directly that chapter. 

### Prerequisites 
- Debian/Ubuntu: `pkg-config`, `build-essential`, `libssl-dev`, `curl`, `jq`, `git`

```
apt install pkg-config build-essential libssl-dev curl jq git
```

- Arch/Manjaro: `base-devel`

```
pacman -S base-devel
```

- Mac OS X: `pkg-config` , `brew`, `openss1`, `protobuf`, `curl`, `git`
Running the following the script installs Homebrew and the above dependencies:

```
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

- `Rust & cargo >= {{minimum_rust_version}}`

We recommend using the [Rust shell script installer](https://www.rust-lang.org/tools/install). Installing cargo from your package manager (e.g. `apt`) is not recommended as the packaged versions are usually too old.

If you really don't want to use the shell script installer, the [Rust installation docs](https://forge.rust-lang.org/infra/other-installation-methods.html) contain instructions for many platforms.

### Download and build Nym binaries
The following commands will compile binaries into the `nym/target/release` directory:

```sh
rustup update
git clone https://github.com/nymtech/nym.git
cd nym
git checkout master # master branch has the latest release version: `develop` will most likely be incompatible with deployed public networks
cargo build --release # build your binaries with **mainnet** configuration
```

Quite a bit of stuff gets built. The key working parts for the workshop are:

* [socks5 client](https://nymtech.net/docs/clients/socks5-client.html): `nym-socks5-client`
* [network requester](https://nymtech.net/operators/nodes/network-requester-setup.html): `nym-network-requester`

## Pre-built Binaries

The [Github releases page](https://github.com/nymtech/nym/releases) has pre-built binaries which should work on Ubuntu 20.04 and other Debian-based systems, but at this stage cannot be guaranteed to work everywhere.

If the pre-built binaries don't work or are unavailable for your system, you will need to build the platform yourself.

All Nym binaries must first be made executable. 

To make a binary executable, open terminal in the same directory and run:

```sh
chmod +x ./<BINARY_NAME> 
# for example: chmod +x ./nym-network-requester
```

### Initialize Sock5 Client and Network Requester

Whether you build the entire platform or downloaded binaries, they need to be initialised with `init` before being `run`.

Navigate in your terminal to the directory where you have your `nym-socks5-client` and `nym-network-requester`. In case you build the entire platform it's in `nym/target/release` - you can change directory from the one where you build by:

```sh
cd target/release
```

The `init` command is usually where you pass flags specifying configuration arguments such as the gateway you wish to communicate with, the ports you wish your binary to listen on, etc. 

The `init` command will also create the necessary keypairs and configuration files at `~/.nym/<BINARY_TYPE>/<BINARY_ID>/` if these files do not already exist. **It will not overwrite existing keypairs if they are present.** 

You can reconfigure your binaries at any time by editing the config file located at `~/.nym/<BINARY_TYPE>/<BINARY_ID>/config/config.toml` and restarting the binary process. 

Once you have run `init`, you can start your binary with the `run` command, usually only accompanied by the `id` of the binary that you specified. 

This `id` is **never** transmitted over the network, and is used to select which local config and key files to use for startup. 

