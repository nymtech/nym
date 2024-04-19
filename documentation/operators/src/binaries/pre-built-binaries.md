# Pre-built Binaries

The [Github releases page](https://github.com/nymtech/nym/releases) has pre-built binaries which should work on Ubuntu 20.04 and other Debian-based systems, but at this stage cannot be guaranteed to work everywhere.

If the pre-built binaries don't work or are unavailable for your system, you will need to build the platform yourself.

## Setup Binaries

> Any syntax in `<>` brackets is a user’s unique variable. Exchange with a corresponding name without the `<>` brackets.

### Download Binary

1. Open [Github releases page](https://github.com/nymtech/nym/releases) and right click on the binary you want
2. Select `Copy Link`
3. Open your VPS terminal in a directory where you want to download Nym binaries.
4. Download binary by running `wget <BINARY_LINK>` where `<BINARY_LINK>` shall be in your clipboard from point \# 2.

### Make Executable

5. Run command:
```sh
chmod +x <BINARY>
# for example: chmod +x nym-mixnode
```
### Run Binary

Now you can use your binary, initialise and run your Nym Node. Follow the guide according to the type of your binary.

**Node setup and usage guides:**

* [Nym Nodes](../nodes/nym-node.md)
* [Validators](../nodes/validator-setup.md)
