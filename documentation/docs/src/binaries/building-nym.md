# Building from Source

> Nym runs on Mac OS X, Linux, and Windows. All nodes **except the Desktop Wallet and NymConnect** on Windows should be considered experimental - it works fine if you're an app developer but isn't recommended for running nodes.

## Building Nym
Nym has two main codebases:

- the [Nym platform](https://github.com/nymtech/nym), written in Rust. This contains all of our code _except_ for the validators.
- the [Nym validators](https://github.com/nymtech/nyxd), written in Go.

> This page details how to build the main Nym platform code. **If you want to build and run a validator, [go here](../nodes/validator-setup.md) instead.**

## Prerequisites
- (Debian/Ubuntu) `pkg-config`, `build-essential`, `libssl-dev`, `curl`, `jq`, `git`

```
sudo apt update
sudo apt install pkg-config build-essential libssl-dev curl jq git
```

- (Arch/Manjaro) `base-devel`

```
pacman -S base-devel
```

- (Mac OS X) `pkg-config` , `brew`, `openss1`, `protobuf`, `curl`, `git`

`curl` - required for transferring data over various protocols. It's used for downloading files or send requests to a server.

`openssl` - used for encryption, decryption, and secure communication over the internet.

`protobuf` - for serialising and deserialising structured data in a fast and efficient way.

Running the following the script installs Homebrew and the above dependencies:

```
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

- `Rust & cargo >= {{minimum_rust_version}}`

We recommend using the [Rust shell script installer](https://www.rust-lang.org/tools/install). Installing cargo from your package manager (e.g. `apt`) is not recommended as the packaged versions are usually too old.

If you really don't want to use the shell script installer, the [Rust installation docs](https://forge.rust-lang.org/infra/other-installation-methods.html) contain instructions for many platforms.

## Download and build Nym binaries
The following commands will compile binaries into the `nym/target/release` directory:

```
rustup update
git clone https://github.com/nymtech/nym.git
cd nym

git reset --hard # in case you made any changes on your branch
git pull # in case you've checked it out before

git checkout release/{{platform_release_version}} # checkout to the latest release branch: `develop` will most likely be incompatible with deployed public networks

cargo build --release # build your binaries with **mainnet** configuration
NETWORK=sandbox cargo build --release # build your binaries with **sandbox** configuration
```

Quite a bit of stuff gets built. The key working parts are:

* [mix node](../nodes/mix-node-setup.md): `nym-mixnode`
* [gateway node](../nodes/gateway-setup.md): `nym-gateway`
* [websocket client](../clients/websocket-client.md): `nym-client`
* [socks5 client](../clients/socks5-client.md): `nym-socks5-client`
* [network requester](../nodes/network-requester-setup.md): `nym-network-requester`
* [nym-cli tool](../tools/nym-cli.md): `nym-cli`

The repository also contains Typescript applications which aren't built in this process. These can be built by following the instructions on their respective docs pages.  
* [Nym Wallet](../wallet/desktop-wallet.md) 
* [Nym Connect](https://nymtech.net/developers/quickstart/nymconnect-gui.html)
* [Network Explorer UI](../explorers/mixnet-explorer.md) 

> You cannot build from GitHub's .zip or .tar.gz archive files on the releases page - the Nym build scripts automatically include the current git commit hash in the built binary during compilation, so the build will fail if you use the archive code (which isn't a Git repository). Check the code out from github using `git clone` instead. 

> You cannot build from GitHub's .zip or .tar.gz archive files on the releases page - the Nym build scripts automatically include the current git commit hash in the built binary during compilation, so the build will fail if you use the archive code (which isn't a Git repository). Check the code out from github using `git clone` instead.
