# NymVPN alpha

Nym has announced [NymVPN](https://nymvpn.com/en) and presented the [NymVPN Litepaper](https://nymvpn.com/nymVPN-litepaper.pdf). At CCC 2023 we have the unique opportunity to do the first alpha public testing. This page provides a how to guide, explaining steps to install and run NymVPN CLI and UI over Nym testnet environment. 
 
NymVPN is a client that uses [Nym Mixnet](https://nymtech.net) to anonymise users entire internet traffic.

The default is 5-hops (including Entry and Exit Gateways)

```
                      ┌─►mix──┐  mix     mix
                      │       │
            Entry     │       │                   Exit
client ───► Gateway ──┘  mix  │  mix  ┌─►mix ───► Gateway ───► internet
                              │       │
                              │       │
                         mix  └─►mix──┘  mix
```

Users can switch to 2-hops only mode, which is a faster but less private option. 

The client can optionally do the first connection to the entry gateway using wireguard, and it uses Mullvad libraries for wrapping `wireguard-go` and to setup local routing rules to route all traffic to the TUN virtual network device.

## Goals

This version is an experimental software for [testing](./nym-vpn.md#testing) purposes in a limited environment. This testing round aims to help Nym with:

* Stabilise NymVPN client
* Understand NymVPN client behaviour with various setups (OS, connectivity, etc.)
* Stabilize the VPN infrastructure and improve its reliability / speed / features (e.g. IPv6 support)
* Load test the network in Sandbox environment and identify / anticipate potential weaknesses
 
 
```admonish info
Our alpha testing round is done live with some participants at CCC 2023. This guide will not work for everyone, as the NymVPN binaries aren't publicly accessible yet. Note that this setup of Nym testnet Sandbox environment is limited for CCC 2023 event and some of the configurations will not work in the future. 

```

FIGURE OUT HOW TO SHARE ACCESS TO DWL THE BINARIES

## Preparation

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

We have CLI and UI binaries available for Linux (Debian based) and macOS. 

![](images/image1.png)

* Visit the [release page](https://github.com/nymtech/nym/releases/) to download the binary for your system.
* Open terminal in the same directory and make executable by running:

```sh
# for CLI
chmod +x ./nym-vpn-cli 

# for UI
chmod +x ./nym-vpn_0.0.0_amd64.AppImage
# make sure your path to package is correct and the package name as well
```


* If you prefer to use the `.deb` version for installation (Linux only), open terminal in the same directory and run:
```
sudo dpkg -i ./<PACKAGE_NAME>.deb
# or
sudo apt-get install -f ./<PACKAGE_NAME>.deb
```

## Running

***For NymVPN to work, all existing VPNs must be switched off!***

* Get your private key for wireguard setup [here](https://nymvpn.com/en/37c3)
* See a JSON list of all Gateways [here](https://nymvpn.com/en/ccc/api/gateways)

### CLI

Make sure your terminal is open in the same directory like your binary.

Running a help command:

```sh
./nym-vpn-cli --help
```

~~~admonish example collapsible=true title="Console output"
```
Usage: nym-vpn-cli [OPTIONS]

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file describing the network
      --mixnet-client-path <MIXNET_CLIENT_PATH>
          Path to the data directory of a previously initialised mixnet client, where the keys reside
      --entry-gateway-id <ENTRY_GATEWAY_ID>
          Mixnet public ID of the entry gateway
      --entry-gateway-country <ENTRY_GATEWAY_COUNTRY>
          Auto-select entry gateway by country ISO
      --exit-router-address <EXIT_ROUTER_ADDRESS>
          Mixnet recipient address
      --exit-gateway-id <EXIT_GATEWAY_ID>

      --exit-router-country <EXIT_ROUTER_COUNTRY>
          Mixnet recipient address
      --enable-wireguard
          Enable the wireguard traffic between the client and the entry gateway
      --private-key <PRIVATE_KEY>
          Associated private key
      --ip <IP>
          The IP address of the TUN device
      --mtu <MTU>
          The MTU of the TUN device
      --disable-routing
          Disable routing all traffic through the VPN TUN device
      --enable-two-hop
          Enable two-hop mixnet traffic. This means that traffic jumps directly from entry gateway to exit gateway
      --enable-poisson-rate
          Enable Poisson process rate limiting of outbound traffic
  -h, --help
          Print help
  -V, --version
          Print version

```
~~~

ADD EXAMPLES FOR RUNNING

ADD GATEWAYS

### UI

If you running NymVPN on mac OS for the first time, you may see this alert message:

![](images/image3.png)

1. Head to System Settings -> Privacy & Security and click `Allow anyway`

![](images/image5.png)

2. Confirm with your password or TouchID

3. Possibly you may have to confirm again upon running the application


## Testing

One of the main aim for the aplha demo is testing. Your share results will help us to make NymVPN robust and stabilise both the client and the network through provided measurements. 

1. Create a directory called `nym-vpn_tests` and copy your `nym-vpn-cli` binary and `sandbox.env` to that directory
2. Copy the [block below](./nym-vpn.md#tests.sh) and save it to the same folder as `tests.sh`
3. Open terminal in the same directory
4. Turn off any existing VPN's and run `sudo sh ./tests.sh`
5. The script will print a JSON view of existing Gateways and prompt you to chose 
    - `enter a gateway ID`: paste one of the values labeled with a key `"identityKey"` (without `" "`)
    - `enter an exit address`: paste one of the values labeled with a key `"address"` (without `" "`)
6. The script shall run the tests and generate a folder called `tests_<LONG_STRING>` and files `perf_test_results.log` or `two_hop_perf_test_results.log` as well as some temp files. This is how the directory structure will look like:
```sh
$ <MY_TESTING_DIRECTORY>
├── tests.sh
├── nym-vpn-cli
├── sandbox.env
├── perf_test_results.log 
├── tests_<LONG_STRING>
│   ├── api_response_times.txt
│   ├── download_time.txt
│   └── ping_results.txt
├── timeout
└── two_hop_perf_test_results.log
```
6. In case of errors, see [troubleshooting section](./nym-vpn.md#troubleshooting) below
7. When the tests are finished, remove the `nym-vpn-cli` binary from the folder and compress it as `nym-vpn_tests.zip`
8. Upload this compressed file to 

ADDD UPLOAD ADDRESS

#### tests.sh

```sh
#!/bin/bash

NEW_ENDPOINT="http://localhost:8000/data.json"

download_file() {
    local file_url=$1
    local output_file=$2
    local time_file=$3

    echo "starting download speed test..."
    local start_time=$(date +%s)
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        wget -O $output_file $file_url
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        curl -o $output_file $file_url
    fi
    local end_time=$(date +%s)
    local elapsed_time=$((end_time - start_time))
    echo "download speed test completed in $elapsed_time seconds." >"$time_file"
}

if ! command -v jq &>/dev/null; then
    echo "jq is not installed. Please install jq to proceed."
    exit 1
fi

data=$(curl -s "$NEW_ENDPOINT")
if [ $? -ne 0 ]; then
    echo "Error fetching data from endpoint"
    exit 1
fi

temp_log_file="temp_log.txt"

perform_tests() {
    local gateway_id=$1
    local exit_address=$2
    local test_directory="tests_${gateway_id}_${exit_address}"
    local file_url="http://ipv4.download.thinkbroadband.com/2MB.zip"

    mkdir -p "$test_directory"
    local ping_results_file="${test_directory}/ping_results.txt"
    local download_time_file="${test_directory}/download_time.txt"
    local api_response_file="${test_directory}/api_response_times.txt"

    # ping test
    echo "starting ping test..."
    for site in google.com youtube.com facebook.com baidu.com wikipedia.org amazon.com twitter.com instagram.com yahoo.com ebay.com netflix.com; do
        ping -c 4 $site >>"$ping_results_file"
    done
    echo "ping test completed. Results saved in $ping_results_file"

    # download speed test
    download_file $file_url /dev/null "$download_time_file"

    # api test
    local api_endpoint="https://validator.nymtech.net/api/v1/mixnodes"
    local iterations=10
    >"$api_response_file"
    for i in $(seq 1 $iterations); do
        local start_time=$(date +%s)
        local response=$(curl -s -o /dev/null -w '%{http_code}' $api_endpoint)
        local end_time=$(date +%s)

        local elapsed_seconds=$((end_time - start_time))
        local hours=$((elapsed_seconds / 3600))
        local minutes=$(((elapsed_seconds % 3600) / 60))
        local seconds=$((elapsed_seconds % 60))

        local human_readable_time=$(printf "%02dh:%02dm:%02ds" $hours $minutes $seconds)
        echo "iteration $i: response Time = ${human_readable_time}, status code = $response" >>"$api_response_file"
    done
    echo "api response test completed. Results saved in $api_response_file."
}

echo $data | jq .

read -p "enter a gateway ID: " identity_key
read -p "enter an exit address: " exit_address

# starting nymVpn
sudo ./nym-vpn-cli-test -c sandbox.env --entry-gateway-id "$identity_key" --exit-router-address "$exit_address" --enable-two-hop >"$temp_log_file" 2>&1 &

timeout=15
start_time=$(date +%s)
while true; do
    current_time=$(date +%s)
    if grep -q "received plain" "$temp_log_file"; then
        echo "successful configuration with identity_key: $identity_key and exit address: $exit_address" >>perf_test_results.log
        perform_tests "$identity_key" "$exit_address"
        break
    fi
    if ((current_time - start_time > timeout)); then
        echo "failed to connect with identity_key: $identity_key using the exit address: $exit_address" >>perf_test_results.log
        break
    fi
    sleep 1
done

echo "terminating nym-vpn-cli..."
pkill -f './nym-vpn-cli'
sleep 5
rm -f "$temp_log_file"
```

## Troubleshooting


