# Binary Initialisation and Configuration

All Nym binaries must first be made executable and initialised with `init` before being `run`. 

To make a binary executable, open terminal in the same directory and run:

```sh
chmod +x <BINARY_NAME> 
# for example: chmod +x nym-mixnode
```

The `init` command is usually where you pass flags specifying configuration arguments such as the gateway you wish to communicate with, the ports you wish your binary to listen on, etc. 

The `init` command will also create the necessary keypairs and configuration files at `~/.nym/<BINARY_TYPE>/<BINARY_ID>/` if these files do not already exist. **It will not overwrite existing keypairs if they are present.** 

You can reconfigure your binaries at any time by editing the config file located at `~/.nym/<BINARY_TYPE>/<BINARY_ID>/config/config.toml` and restarting the binary process. 

Once you have run `init`, you can start your binary with the `run` command, usually only accompanied by the `id` of the binary that you specified. 

This `id` is **never** transmitted over the network, and is used to select which local config and key files to use for startup. 
