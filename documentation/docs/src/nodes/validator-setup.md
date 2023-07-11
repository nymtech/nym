# Validators

The validator is built using [Cosmos SDK](https://cosmos.network) and [Tendermint](https://tendermint.com), with a [CosmWasm](https://cosmwasm.com) smart contract controlling the directory service, node bonding, and delegated mixnet staking.

## Building your validator
### Prerequisites
#### `git`, `gcc`, `jq`
* Debian-based systems:
```
apt install git build-essential jq

# optional additional manual pages can be installed with:
apt-get install manpages-dev
```

* Arch-based systems:
Install `git`, `gcc` and `jq` with the following:
```
pacman -S git gcc jq
```

#### `Go`
`Go` can be installed via the following commands (taken from the [Agoric SDK docs](https://github.com/Agoric/agoric-sdk/wiki/Validator-Guide-for-Incentivized-Testnet#install-go)):

```
# First remove any existing old Go installation
sudo rm -rf /usr/local/go

# Install correct Go version
curl https://dl.google.com/go/go1.20.4.linux-amd64.tar.gz | sudo tar -C/usr/local -zxvf -

# Update environment variables to include go
cat <<'EOF' >>$HOME/.profile
export GOROOT=/usr/local/go
export GOPATH=$HOME/go
export GO111MODULE=on
export PATH=$PATH:/usr/local/go/bin:$HOME/go/bin
EOF
source $HOME/.profile
```

Verify `Go` is installed with:

```
go version
# Should return something like:
go version go1.20.4 linux/amd64
```

### Download a precompiled validator binary
You can find pre-compiled binaries for Ubuntu `22.04` and `20.04` [here](https://github.com/nymtech/nyxd/releases).

```admonish caution title=""
There are seperate releases for Mainnet and the Sandbox testnet - make sure to download the correct binary to avoid `bech32Prefix` mismatches.
```

### Manually compiling your validator binary
The codebase for the Nyx validators can be found [here](https://github.com/nymtech/nyxd).

The validator binary can be compiled by running the following commands:
```
git clone https://github.com/nymtech/nyxd.git
cd nyxd
git checkout release/<nyxd_version>

# Mainnet
make build

# Sandbox testnet
BECH32_PREFIX=nymt make build
```

At this point, you will have a copy of the `nyxd` binary in your `build/` directory. Test that it's compiled properly by running:

```
./build/nyxd
```

You should see help text print out.

### Linking `nyxd` to `libwasmvm.so`

`libwasmvm.so` is the wasm virtual machine which is needed to execute smart contracts in `v0.26.1`. This file is renamed in `libwasmvm.x86_64.so` in `v0.31.1`.

If you downloaded your `nyxd` binary from Github, you will have seen this file when un-`tar`-ing the `.tar.gz` file from the releases page.

If you are seeing an error concerning this file when trying to run `nyxd`, then you need to move the `libwasmvm.so` file to correct location.

Simply `cp` or `mv` that file to `/lib/x86_64-linux-gnu/` and re-run `nyxd`.

### Adding `nyxd` to your `$PATH`
You'll need to set `LD_LIBRARY_PATH` in your user's `~/.bashrc` file, and add that to our path. Replace `/home/youruser/path/to/nym/binaries` in the command below to the locations of `nyxd` and `libwasmvm.so` and run it. If you have compiled these on the server, they will be in the `build/` folder:

```
NYX_BINARIES=/home/youruser/path/to/validator/binary

# if you are using another shell like zsh replace '.bashrc' with the relevant config file
echo 'export LD_LIBRARY_PATH=${LD_LIBRARY_PATH}:'NYX_BINARIES >> ~/.bashrc
echo 'export PATH=$PATH:'${NYX_BINARIES} >> ~/.bashrc
source ~/.bashrc
```

Test everything worked:

```
nyxd
```

This should return the regular help text.

## Initialising your validator
### Prerequisites:

- FQDN Domain name
- IPv4 and IPv6 connectivity

Choose a name for your validator and use it in place of `<ID>` in the following command:

```
# Mainnet
nyxd init <ID> --chain-id=nyx

# Sandbox testnet
nyxd init <ID> --chain-id=sandbox
```

```admonish caution title=""
`init` generates `priv_validator_key.json` and `node_key.json`.

If you have already set up a validator on a network, **make sure to back up the key located at**
`~/.nyxd/config/priv_validator_key.json`.

If you don't save the validator key, then it can't sign blocks and will be jailed all the time, and
there is no way to deterministically (re)generate this key.
```

At this point, you have a new validator, with its own genesis file located at `$HOME/.nyxd/config/genesis.json`. You will need to replace the contents of that file that with either the Nyx Mainnet or Sandbox Testnet genesis file.

You can use the following command to download them for the correct network:

```
# Mainnet
wget  -O $HOME/.nyxd/config/genesis.json https://nymtech.net/genesis/genesis.json

# Sandbox testnet
wget -O $HOME/.nyxd/config/genesis.json https://sandbox-validator1.nymtech.net/snapshots/genesis.json
```

### `config.toml` configuration

Edit the following config options in `$HOME/.nyxd/config/config.toml` to match the information below for your network:

```
# Mainnet
persistent_peers = "ee03a6777fb76a2efd0106c3769daaa064a3fcb5@51.79.21.187:26656"
create_empty_blocks = false
laddr = "tcp://0.0.0.0:26656"
```

```
# Sandbox testnet
cors_allowed_origins = ["*"]
persistent_peers = "8421c0a3d90d490e27e8061f2abcb1276c8358b6@sandbox-validator1.nymtech.net:26666"
create_empty_blocks = false
laddr = "tcp://0.0.0.0:26656"
```

These affect the following:
* `persistent_peers = "<PEER_ADDRESS>@<DOMAIN>.nymtech.net:26666"` allows your validator to start pulling blocks from other validators. **The main sandbox validator listens on `26666` instead of the default `26656` for debugging**. It is recommended you do not change your port from `26656`.
* `create_empty_blocks = false` will save space
* `laddr = "tcp://0.0.0.0:26656"` is in your p2p configuration options

Optionally, if you want to enable [Prometheus](https://prometheus.io/) metrics then the following must also match in the `config.toml`:

- `prometheus = true`
- `prometheus_listen_addr = ":26660"`

> Remember to enable metrics in the 'Configuring Prometheus metrics' section below as well.

And if you wish to add a human-readable moniker to your node:

- `moniker = "yourname"`

Finally, if you plan on using [Cockpit](https://cockpit-project.org/documentation.html) on your server, change the `grpc` port from `9090` as this is the port used by Cockpit.

### `app.toml` configuration
In the file `$HOME/nyxd/config/app.toml`, set the following values:

```
# Mainnet
minimum-gas-prices = "0.025unym,0.025unyx"
enable = true in the `[api]` section to get the API server running
```
```
# Sandbox Testnet
minimum-gas-prices = "0.025unymt,0.025unyxt"
enable = true` in the `[api]` section to get the API server running
```

### Setting up your validator's admin user
You'll need an admin account to be in charge of your validator. Set that up with:

```
nyxd keys add nyxd-admin
```

This will add keys for your administrator account to your system's keychain and log your name, address, public key, and mnemonic. As the instructions say, remember to **write down your mnemonic**.

You can get the admin account's address with:

```
nyxd keys show nyxd-admin -a
```

Type in your keychain **password**, not the mnemonic, when asked.

## Starting your validator

```admonish caution title=""
If you are running a Sandbox testnet validator, please skip the `validate-genesis` command: it will fail due to the size of the genesis file as this is a fork of an existing chain state.
```

Everything should now be ready to go. You've got the validator set up, all changes made in `config.toml` and `app.toml`, the Nym genesis file copied into place (replacing the initial auto-generated one). Now let's validate the whole setup:

```
nyxd validate-genesis
```

If this check passes, you should receive the following output:

```
File at /path/to/genesis.json is a valid genesis file
```

> If this test did not pass, check that you have replaced the contents of `/path/to/.nymd/config/genesis.json` with that of the correct genesis file.

Before starting the validator, we will need to open the firewall ports:

```
# if ufw is not already installed:
sudo apt install ufw
sudo ufw enable
sudo ufw allow 1317,26656,26660,22,80,443/tcp
# to check everything worked
sudo ufw status
```

Ports `22`, `80`, and `443` are for ssh, http, and https connections respectively. The rest of the ports are documented [here](https://docs.cosmos.network/v0.42/core/grpc_rest.html).

For more information about your validator's port configuration, check the [validator port reference table](#validator-port-reference) below.

> If you are planning to use [Cockpit](https://cockpit-project.org/) on your validator server then you will have defined a different `grpc` port in your `config.toml` above: remember to open this port as well.

Start the validator:

```
nyxd start
```

Once your validator starts, it will start requesting blocks from other validators. This may take several hours. Once it's up to date, you can issue a request to join the validator set with the command below.

### Syncing from a snapshot
If you wish to sync from a snapshot on **mainnet** use Polkachu's [mainnet](https://polkachu.com/networks/nym) resources.

If you wish to sync from a snapshot on **Sandbox testnet** use the below commands, which are a modified version of Polkachu's excellent resources. These commands assume you are running an OS with `apt` as the package manager:

```
# install lz4 if necessary
sudo apt install snapd -y
sudo snap install lz4

# download the snapshot
wget -O nyxd-sandbox-snapshot-data.tar.lz4 https://sandbox-validator1.nymtech.net/snapshots/nyxd-sandbox-snapshot-data.tar.lz4

# reset your validator state
nyxd tendermint unsafe-reset-all

# unpack the snapshot
lz4 -c -d nyxd-sandbox-snapshot-data.tar.lz4 | tar -x -C $HOME/.nyxd
```

You can then restart `nyxd` - it should start syncing from a block > 2000000.

### Joining Consensus
```admonish caution title=""
When joining consensus, make sure that you do not disrupt (or worse - halt) the network by coming in with a disproportionately large amount of staked tokens.

Please initially stake a small amount of tokens compared to existing validators, then delegate to yourself in tranches over time.
```

Once your validator has synced and you have received tokens, you can join consensus and produce blocks.

```
# Mainnet
nyxd tx staking create-validator
  --amount=10000000unyx
  --fees=0unyx
  --pubkey=$(/home/youruser/path/to/nyxd/binaries/nyxd tendermint show-validator)
  --moniker="whatever you called your validator"
  --chain-id=nyx
  --commission-rate="0.10"
  --commission-max-rate="0.20"
  --commission-max-change-rate="0.01"
  --min-self-delegation="1"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --node https://rpc-1.nyx.nodes.guru:443
```
```
# Sandbox Testnet
nyxd tx staking create-validator
  --amount=10000000unyxt
  --fees=5000unyxt
  --pubkey=$(/home/youruser/path/to/nym/binaries/nyxd tendermint show-validator)
  --moniker="whatever you called your validator"
  --chain-id=sandbox
  --commission-rate="0.10"
  --commission-max-rate="0.20"
  --commission-max-change-rate="0.01"
  --min-self-delegation="1"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --node https://sandbox-validator1.nymtech.net:443
```

You'll need either `unyxt` tokens on Sandbox, or `unyx` tokens on mainnet to perform this command.

> We are currently working towards building up a closed set of reputable validators. You can ask us for coins to get in, but please don't be offended if we say no - validators are part of our system's core security and we are starting out with people we already know or who have a solid reputation.

If you want to edit some details for your node you will use a command like this:

```
# Mainnet
nyxd tx staking edit-validator
  --chain-id=nyx
  --moniker="whatever you called your validator"
  --details="Nyx validator"
  --security-contact="your email"
  --identity="your identity"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --fees 2000unyx
```
```
# Sandbox testnet
nyxd tx staking edit-validator
  --chain-id=sandbox
  --moniker="whatever you called your validator"
  --details="Sandbox testnet validator"
  --security-contact="your email"
  --identity="your identity"
  --gas="auto"
  --gas-adjustment=1.15
  --from="KEYRING_NAME"
  --fees 2000unyxt
```

With above command you can specify the `gpg` key last numbers (as used in `keybase`) as well as validator details and your email for security contact.

### Automating your validator with systemd
You will most likely want to automate your validator restarting if your server reboots. Below is a systemd unit file to place at `/etc/systemd/system/nymd.service`:

```ini
[Unit]
Description=Nyxd
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nyx                                                          # change to your user
Type=simple
Environment="LD_LIBRARY_PATH=/home/youruser/path/to/nyx/binaries" # change to correct path
ExecStart=/home/youruser/path/to/nyx/binaries/nymd start          # change to correct path
Restart=on-failure
RestartSec=30
LimitNOFILE=infinity

[Install]
WantedBy=multi-user.target
```

Proceed to start it with:

```
systemctl daemon-reload # to pickup the new unit file
systemctl enable nymd   # to enable the service
systemctl start nymd    # to actually start the service
journalctl -f           # to monitor system logs showing the service start
```

### Installing and configuring nginx for HTTPS
#### Setup
[Nginx](https://www.nginx.com/resources/glossary/nginx/#:~:text=NGINX%20is%20open%20source%20software,%2C%20media%20streaming%2C%20and%20more.&text=In%20addition%20to%20its%20HTTP,%2C%20TCP%2C%20and%20UDP%20servers.) is an open source software used for operating high-performance web servers. It allows us to set up reverse proxying on our validator server to improve performance and security.

Install `nginx` and allow the 'Nginx Full' rule in your firewall:

```
sudo ufw allow 'Nginx Full'
```

Check nginx is running via systemctl:

```
systemctl status nginx
```

Which should return:

```
● nginx.service - A high performance web server and a reverse proxy server
   Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)
   Active: active (running) since Fri 2018-04-20 16:08:19 UTC; 3 days ago
     Docs: man:nginx(8)
 Main PID: 2369 (nginx)
    Tasks: 2 (limit: 1153)
   CGroup: /system.slice/nginx.service
           ├─2369 nginx: master process /usr/sbin/nginx -g daemon on; master_process on;
           └─2380 nginx: worker process
```

#### Configuration

Proxying your validator's port `26657` to nginx port `80` can then be done by creating a file with the following at `/etc/nginx/conf.d/validator.conf`:

```
server {
  listen 80;
  listen [::]:80;
  server_name "domain_name";

  location / {
    proxy_pass http://127.0.0.1:26657;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
  }
}
```

Followed by:

```
sudo apt install certbot nginx python3
certbot --nginx -d nym-validator.yourdomain.com -m you@yourdomain.com --agree-tos --noninteractive --redirect
```

```admonish caution title=""
If using a VPS running Ubuntu 20: replace `certbot nginx python3` with `python3-certbot-nginx`
```

These commands will get you an https encrypted nginx proxy in front of the API.

### Configuring Prometheus metrics (optional)

Configure Prometheus with the following commands (adapted from NodesGuru's [Agoric setup guide](https://nodes.guru/agoric/setup-guide/en)):

```
echo 'export OTEL_EXPORTER_PROMETHEUS_PORT=9464' >> $HOME/.bashrc
source ~/.bashrc
sed -i '/\[telemetry\]/{:a;n;/enabled/s/false/true/;Ta}' $HOME/.nymd/config/app.toml
sed -i "s/prometheus-retention-time = 0/prometheus-retention-time = 60/g" $HOME/.nymd/config/app.toml
sudo ufw allow 9464
echo 'Metrics URL: http://'$(curl -s ifconfig.me)':26660/metrics'
```

Your validator's metrics will be available to you at the returned 'Metrics URL'.

~~~admonish example collapsible=true title="Console output"
```
# HELP go_gc_duration_seconds A summary of the pause duration of garbage collection cycles.
# TYPE go_gc_duration_seconds summary
go_gc_duration_seconds{quantile="0"} 6.7969e-05
go_gc_duration_seconds{quantile="0.25"} 7.864e-05
go_gc_duration_seconds{quantile="0.5"} 8.4591e-05
go_gc_duration_seconds{quantile="0.75"} 0.000115919
go_gc_duration_seconds{quantile="1"} 0.001137591
go_gc_duration_seconds_sum 0.356555301
go_gc_duration_seconds_count 2448
# HELP go_goroutines Number of goroutines that currently exist.
# TYPE go_goroutines gauge
go_goroutines 668
# HELP go_info Information about the Go environment.
# TYPE go_info gauge
go_info{version="go1.15.7"} 1
# HELP go_memstats_alloc_bytes Number of bytes allocated and still in use.
# TYPE go_memstats_alloc_bytes gauge
go_memstats_alloc_bytes 1.62622216e+08
# HELP go_memstats_alloc_bytes_total Total number of bytes allocated, even if freed.
# TYPE go_memstats_alloc_bytes_total counter
go_memstats_alloc_bytes_total 2.09341707264e+11
# HELP go_memstats_buck_hash_sys_bytes Number of bytes used by the profiling bucket hash table.
# TYPE go_memstats_buck_hash_sys_bytes gauge
go_memstats_buck_hash_sys_bytes 5.612319e+06
# HELP go_memstats_frees_total Total number of frees.
# TYPE go_memstats_frees_total counter
go_memstats_frees_total 2.828263344e+09
# HELP go_memstats_gc_cpu_fraction The fraction of this program's available CPU time used by the GC since the program started.
# TYPE go_memstats_gc_cpu_fraction gauge
go_memstats_gc_cpu_fraction 0.03357798610671518
# HELP go_memstats_gc_sys_bytes Number of bytes used for garbage collection system metadata.
# TYPE go_memstats_gc_sys_bytes gauge
go_memstats_gc_sys_bytes 1.3884192e+07
```
~~~

### Setting the ulimit
Linux machines limit how many open files a user is allowed to have. This is called a `ulimit`.

`ulimit` is 1024 by default on most systems. It needs to be set higher, because validators make and receive a lot of connections to other nodes.

If you see errors such as:

```
Failed to accept incoming connection - Os { code: 24, kind: Other, message: "Too many open files" }
```

This means that the operating system is preventing network connections from being made.

##### Set the ulimit via `systemd` service file
Query the `ulimit` of your validator with:

```
grep -i "open files" /proc/$(ps -A -o pid,cmd|grep nymd | grep -v grep |head -n 1 | awk '{print $1}')/limits
```

You'll get back the hard and soft limits, which looks something like this:

```
Max open files            65536                65536                files
```

If your output is **the same as above**, your node will not encounter any `ulimit` related issues.

However if either value is `1024`, you must raise the limit via the systemd service file. Add the line:

```
LimitNOFILE=65536
```

Reload the daemon:

```
systemctl daemon-reload
```

or execute this as root for system-wide setting of `ulimit`:

```
echo "DefaultLimitNOFILE=65535" >> /etc/systemd/system.conf
```

Reboot your machine and restart your node. When it comes back, use `cat /proc/$(pidof nym-validator)/limits | grep "Max open files"` to make sure the limit has changed to 65535.

##### Set the ulimit on `non-systemd` based distributions
Edit `etc/security/conf` and add the following lines:

```
# Example hard limit for max opened files
username        hard nofile 4096
# Example soft limit for max opened files
username        soft nofile 4096
```

Then reboot your server and restart your validator.

## Using your validator
### Unjailing your validator
If your validator gets jailed, you can fix it with the following command:

```
# Mainnet
nyxd tx slashing unjail
  --broadcast-mode=block
  --from="KEYRING_NAME"
  --chain-id=nyx
  --gas=auto
  --gas-adjustment=1.4
  --fees=7000unyx
```
```
# Sandbox Testnet
nyxd tx slashing unjail
  --broadcast-mode=block
  --from="KEYRING_NAME"
  --chain-id=sandbox
  --gas=auto
  --gas-adjustment=1.4
  --fees=7000unyxt
```

### Upgrading your validator

Upgrading from `v0.31.1` -> `v0.32.0` process is fairly simple. Grab the `v0.32.0` release tarball from the [`nyxd` releases page](https://github.com/nymtech/nyxd/releases), and untar it. Inside are two files:

- the new validator (`nyxd`) v0.32.0
- the new wasmvm (it depends on your platform, but most common filename is `libwasmvm.x86_64.so`)

Wait for the upgrade height to be reached and the chain to halt awaiting upgrade, then:

* copy `libwasmvm.x86_64.so` to the default LD_LIBRARY_PATH on your system (on Ubuntu 20.04 this is `/lib/x86_64-linux-gnu/`) replacing your existing file with the same name.
* swap in your new `nyxd` binary and restart.

You can also use something like [Cosmovisor](https://github.com/cosmos/cosmos-sdk/tree/main/tools/cosmovisor) - grab the relevant information from the current upgrade proposal [here](https://nym.explorers.guru/proposal/9).

Note: Cosmovisor will swap the `nyxd` binary, but you'll need to already have the `libwasmvm.x86_64.so` in place.

#### Common reasons for your validator being jailed

The most common reason for your validator being jailed is that your validator is out of memory because of bloated syslogs.

Running the command `df -H` will return the size of the various partitions of your VPS.

If the `/dev/sda` partition is almost full, try pruning some of the `.gz` syslog archives and restart your validator process.

### Day 2 operations with your validator

You can check your current balances with:

```
nymd query bank balances ${ADDRESS}
```

For example, on the Sanbox testnet this would return:

```yaml
balances:
- amount: "919376"
denom: unymt
pagination:
next_key: null
total: "0"
```

You can, of course, stake back the available balance to your validator with the following command.

> Remember to save some tokens for gas costs!

```
# Mainnet
nyxd tx staking delegate VALOPERADDRESS AMOUNTunym
  --from="KEYRING_NAME"
  --keyring-backend=os
  --chain-id=nyx
  --gas="auto"
  --gas-adjustment=1.15
  --fees 5000unyx
```
```
# Sandbox Testnet
nyxd tx staking delegate VALOPERADDRESS AMOUNTunymt
  --from="KEYRING_NAME"
  --keyring-backend=os
  --chain-id=sandbox
  --gas="auto"
  --gas-adjustment=1.15
  --fees 5000unyxt
```

## Validator port reference
All validator-specific port configuration can be found in `$HOME/.nymd/config/config.toml`. If you do edit any port configs, remember to restart your validator.

| Default port | Use                                  |
|--------------|--------------------------------------|
| 1317         | REST API server endpoint             |
| 26656        | Listen for incoming peer connections |
| 26660        | Listen for Prometheus connections    |
