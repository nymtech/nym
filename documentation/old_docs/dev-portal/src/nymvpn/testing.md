# Testing NymVPN alpha

<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://player.vimeo.com/video/908640440?h=0f7f6dfa53" style="position:absolute;top:0;left:0;width:100%;height:100%;" frameborder="0" allow="autoplay; fullscreen; picture-in-picture" allowfullscreen></iframe></div><script src="https://player.vimeo.com/api/player.js"></script>

```admonish info
NymVPN is an experimental software and it's for [testing](./testing.md) purposes only. All users testing the client are expected to sign GDPR Information Sheet and Consent Form (shared at the workshop) so we use their results to improve the client, and submit the form [*NymVPN User research*]({{nym_vpn_form_url}}) with the testing results.
```

> Before you get into testing NymVPN, make sure to go through the preparation steps for NymVPN [CLI](cli.md).

One of the main aims of NymVPN alpha release is testing; your results will help us to make NymVPN robust and stabilise both the client and the network through provided measurements.

## Steps to test NymVPN

> Any syntax in `<>` brackets is a user's/version unique variable. Exchange with a corresponding name without the `<>` brackets.

1. Create a directory called `nym-vpn-tests` and copy your `nym-vpn-cli` binary ([download here]({{nym_vpn_releases}}))
2. Copy or download [`sandbox.env`](https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env) testnet config file to the same directory
```sh
curl -o sandbox.env -L https://raw.githubusercontent.com/nymtech/nym/develop/envs/sandbox.env
```
3. Copy the [block below](#testssh) and save it as `tests.sh` to the same folder
4. Open terminal in the same directory and make the script executable
```sh
chmod u+x ./tests.sh
```
5. Turn off any existing VPN's (including NymVPN instances), reconnect your wifi and run the `tests.sh` script
```sh
sudo ./tests.sh
````
6. In case of errors, see the [troubleshooting section](troubleshooting.md#missing-jq-error)
7. The script will print a JSON view of existing Gateways and prompt you to:
    - *Make sure to use two different Gateways for entry and exit!*
    - `enter a gateway ID:` paste one of the values labeled with a key `"identityKey"` printed above (without `" "`)
    - `enter an exit address:` paste one of the values labeled with a key `"address"` printed above (without `" "`)
    - `enable WireGuard? (yes/no):` if you chose yes, find your private key and wireguard IP [here](https://nymvpn.com/en/alpha)
8. Note that the testing script doesn't print many logs, in case of doubts you can check logs in the log file `temp_log.txt` located in the same directory.
9. The script shall run the tests and generate a folder called `tests_<LONG_STRING>` and files `perf_test_results.log` or `two_hop_perf_test_results.log` as well as some temp files. This is how the directory structure will look like:
```sh
nym-vpn-tests
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
10. When the tests are finished, remove the `nym-vpn-cli` binary from the folder and compress the entire folder as `nym-vpn-tests.zip` (both of these can be done in your graphical environment)
11. Upload this compressed file to the [form]({{nym_vpn_form_url}}) drop field when prompted

#### tests.sh

This is the testing script which needs to be copied and saved as `tests.sh` to your `nym-vpn-tests` folder and then run from there as described [above](#steps-to-test-nymvpn).

```sh
#!/bin/bash

ENDPOINT="https://sandbox-nym-api1.nymtech.net/api/v1/gateways/described"
json_array=()
echo "🚀 🏎 - please be patient, i'm fetching you your entry points - 🚀 🏎 "

data=$(curl -s "$ENDPOINT" | jq -c '.[] | {host: .bond.gateway.host, hostname: .self_described.host_information.hostname, identity_key: .bond.gateway.identity_key, exitGateway: .self_described.ip_packet_router.address}')

while IFS= read -r entry; do
    host=$(echo "$entry" | jq -r '.host')
    hostname=$(echo "$entry" | jq -r '.hostname')
    identity_key=$(echo "$entry" | jq -r '.identity_key')
    exit_gateway_address=$(echo "$entry" | jq -r '.exitGateway // empty')
    valid_ip=$(echo "$host")

    if [ -n "$exit_gateway_address" ]; then
        exit_gateway="{\"address\": \"${exit_gateway_address}\"}"
    else
        exit_gateway="{}"
    fi
    if [[ $valid_ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        country_info=$(curl -s "http://ipinfo.io/${valid_ip}/country" | tr -d '\n')
        country_info_escaped=$(echo "$country_info" | tr -d '\n' | jq -aRs . | tr -d '"')
    else
        country_info_escaped=""
    fi
    json_object="{\"hostname\": \"${hostname}\", \"identityKey\": \"${identity_key}\", \"exitGateway\": ${exit_gateway}, \"location\": \"${country_info_escaped}\"}"
    json_array+=("$json_object")
done < <(echo "$data")

if [ $? -ne 0 ]; then
    echo "error fetching data from endpoint"
    exit 1
fi

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

printf "%s\n" "${json_array[@]}" | jq -s .

read -p "enter a gateway ID: " identity_key
read -p "enter an exit address: " exit_address

while true; do
    read -p "enable WireGuard? (yes/no): " enable_wireguard
    enable_wireguard=$(echo "$enable_wireguard" | tr '[:upper:]' '[:lower:]')

    case "$enable_wireguard" in
    "yes")
        read -p "enter your WireGuard private key: " priv_key
        read -p "enter your WireGuard IP: " wg_ip
        wireguard_options="--enable-wireguard --private-key $priv_key --wg-ip $wg_ip"
        break
        ;;
    "no")
        wireguard_options=""
        break
        ;;
    *)
        echo "invalid response. please enter 'yes' or 'no'."
        ;;
    esac
done

sudo ./nym-vpn-cli -c sandbox.env --entry-gateway-id ${identity_key} --exit-router-address ${exit_address} --enable-two-hop $wireguard_options >"$temp_log_file" 2>&1 &

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
