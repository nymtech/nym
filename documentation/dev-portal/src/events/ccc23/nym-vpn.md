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

* Get your private key for wireguard [here](https://nymvpn.com/en/37c3)
* See a JSON list of all Gateways [here](https://nymvpn.com/en/37c3/api/gateways)

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

1. Copy the block below and save it to the same folder where you have your `nym-vpn-cli` binary as `tests.sh`
2. Open terminal in the same directory
3. Run `sudo sh ./tests.sh`
4. ADD HOW TO SHARE RESULTS

```sh
#!/bin/bash

json='[
  {
    "host": "143.42.96.234",
    "identity_key": "378R4NXg38GESird5LYTyz7pZ5PFXXpqCxSjRCq9Jg7J",
    "address": "6w1zY8LGsw97H92KEdjCMvEDZoKSvXiLnFzLhCnJHmqt.3urfUjH6QG3R8va4pW3vmP2cLtFEUZcofhKuwmHdE8X6@378R4NXg38GESird5LYTyz7pZ5PFXXpqCxSjRCq9Jg7J",
    "country": "GB",
    "distance_to_entry_gateway": "901km"
  },
  {
    "host": "85.159.212.96",
    "identity_key": "5UCiizgbjBQoJYpZ7te6xUoMLjJQrQQRSvsD6tcBPH2n",
    "address": "9fM5gvHU7xSdZYKE6qAMG7j4Ps2xLeV8Z9cwCLGFv1Gx.HDuXHaFciHQRT63zteyJYk9Vns2LA82v2ABKpcQt1EyS@5UCiizgbjBQoJYpZ7te6xUoMLjJQrQQRSvsD6tcBPH2n",
    "country": "GB",
    "distance_to_entry_gateway": "901km"
  },
  {
    "host": "176.58.120.72",
    "identity_key": "666hA2R52Jmasbx9H1S7DzcGE6x7s7pSxSSB6pWqMKHE",
    "address": "FaVTCU2m9G18CVGofaQaXk19vfW4VVtXVEogS54fHT53.7PR8kg6nXGe5WgCJ4x6VkNzZTkfKhXLowSWA9Lg4BsPj@666hA2R52Jmasbx9H1S7DzcGE6x7s7pSxSSB6pWqMKHE",
    "country": "GB",
    "distance_to_entry_gateway": "901km"
  },
  {
    "host": "172.232.36.90",
    "identity_key": "7LzcTUZM91MsYSmthN6up4KC9vFf2dmtXwBPhF7W3QM5",
    "address": "3qoMx9S39ZVXXmmyS8y5Qp5RWQKrFsvge5ftHS3okVq4.G7NaZswPAjtHbBNKmyGW2pnmt51GCtyn3gULEy1FnR6T@7LzcTUZM91MsYSmthN6up4KC9vFf2dmtXwBPhF7W3QM5",
    "country": "FR",
    "distance_to_entry_gateway": "584km"
  },
  {
    "host": "51.20.115.58",
    "identity_key": "AMAQ2LCzyxqdejn4nZsfz94gK11K2sRwkKek6oVFm1WB",
    "address": "CxStdSUsAsLeiGktFWSDb7X3dSEAqSvszMCK2J9HB6rq.EEQfesdCwSyreqcSsdtpWfKgcftSTnTv13oLn6GmvQae@AMAQ2LCzyxqdejn4nZsfz94gK11K2sRwkKek6oVFm1WB",
    "country": "SE",
    "distance_to_entry_gateway": "1601km"
  },
  {
    "host": "172.232.134.126",
    "identity_key": "AnCe6phpAp3ne2gT3rwNQ4vH6QTNBggPhNka4hBQrEUY",
    "address": "H8KiTzNAVBuoqA8hDiTmfpC7duZerew4LHRCU6KtWsYo.4KniYqvavPVmMNYqMiHw1gFtHoWCsu8FRLZX31D2bVsJ@AnCe6phpAp3ne2gT3rwNQ4vH6QTNBggPhNka4hBQrEUY",
    "country": "SE",
    "distance_to_entry_gateway": "1601km"
  },
  {
    "host": "3.250.81.180",
    "identity_key": "BHsWt4DEKERkuEgkKburBU81MpDcYk8KPxXR7URNqP78",
    "address": "7CAFAFofs28BYudM685iTtpZzCuNzcWn4gApxiX7VLTa.7fiUcee2tMJy5XBQAXsECRA1zM9tUYGcLBcArshe1PTS@BHsWt4DEKERkuEgkKburBU81MpDcYk8KPxXR7URNqP78",
    "country": "IE",
    "distance_to_entry_gateway": "1361km"
  },
  {
    "host": "35.181.57.111",
    "identity_key": "CSwbNyC9Tb8HSMQU5EjVqByNNLkkj6aBBPtQFBdVEHFa",
    "address": "5TpMKoWFQSqyUw4NKSgph6zgW61zcvus73C17J3ZozKz.AzXRZ2cUGVzsRDeWiwK3bqJh3LskbFadK3akTbeAnX2@CSwbNyC9Tb8HSMQU5EjVqByNNLkkj6aBBPtQFBdVEHFa",
    "country": "FR",
    "distance_to_entry_gateway": "589km"
  },
  {
    "host": "139.162.180.253",
    "identity_key": "DphEmo33pZonPcBwJxFEkdS1EMKS9uAt7VGZVdsxGBgY",
    "address": "4T3BGyjUFZDp5iZa7kGPzx2Kq5UNzQSomwwv4w7D7rGF.56kCcEMvAUsHSZ2CXuJv8Wp4vBwg4CFDTSP4N5Eogx5d@DphEmo33pZonPcBwJxFEkdS1EMKS9uAt7VGZVdsxGBgY",
    "country": "DE",
    "distance_to_entry_gateway": "458km"
  },
  {
    "host": "13.39.161.56",
    "identity_key": "DumEE4vMPrak6oRTSGwwyiYsPFRqtJjy26WEXVNEnZrg",
    "address": "4WVqE2C1zRWNVZqD6vthDg9FntRDyvhwEwdxajQGUMYc.2kBhfFiwknJex3jyeEqKtLNRANeWryNghHvSLJPwRcfS@DumEE4vMPrak6oRTSGwwyiYsPFRqtJjy26WEXVNEnZrg",
    "country": "FR",
    "distance_to_entry_gateway": "589km"
  },
  {
    "host": "170.187.187.235",
    "identity_key": "EUFhawe7PgYWbXVhv1PcBfeEoNYTNPPE1HXKRELR7bN8",
    "address": "JAQ4PXuf2FvrTqjan25T1zrNLdxm2jGqwDZZL6a5T7h2.7618iBtCMGhZAteRK16YVowAgnj7wr978Ff4Qbo8Tyvr@EUFhawe7PgYWbXVhv1PcBfeEoNYTNPPE1HXKRELR7bN8",
    "country": "DE",
    "distance_to_entry_gateway": "458km"
  }
]'

cleanup() {
  echo "terminating nym-vpn-cli..."
  pkill -f './nym-vpn-cli'
  sleep 5
}

temp_log_file="temp_log.txt"

perform_tests() {
  #------------------------------------------------------------------------
  # ping test
  #------------------------------------------------------------------------
  gateway_id=$1
  exit_address=$2

  test_directory="tests_${gateway_id}_${exit_address}"
  mkdir -p "$test_directory"

  sites=(google.com youtube.com facebook.com baidu.com wikipedia.org
    amazon.com twitter.com instagram.com yahoo.com ebay.com netflix.com)

  echo "starting ping test..."
  ping_results_file="${test_directory}/ping_results_${gateway_id}_${exit_address}.txt"
  for site in "${sites[@]}"; do
    ping -c 4 $site >>"$ping_results_file"
  done
  echo "ping test completed. Results saved in $ping_results_file"

  #------------------------------------------------------------------------

  file_url="http://ipv4.download.thinkbroadband.com/2MB.zip"
  wget_time_file="${test_directory}/wget_time_${gateway_id}_${exit_address}.txt"
  curl_time_file="${test_directory}/curl_time_${gateway_id}_${exit_address}.txt"

  echo "starting download speed test with wget..."
  start_time=$(date +%s)
  wget -O /dev/null $file_url
  end_time=$(date +%s)
  wget_time=$((end_time - start_time))
  echo "download speed test with wget completed in $wget_time seconds." > "$wget_time_file"

  echo "starting download speed test with curl..."
  start_time=$(date +%s)
  curl -o /dev/null $file_url
  end_time=$(date +%s)
  curl_time=$((end_time - start_time))
  echo "download speed test with curl completed in $curl_time seconds." >"$curl_time_file"

  #------------------------------------------------------------------------
  # api test
  api_endpoint="https://validator.nymtech.net/api/v1/mixnodes"
  iterations=10
  api_response_file="${test_directory}/api_response_times_${gateway_id}_${exit_address}.txt"
  >"$api_response_file"
  for i in $(seq 1 $iterations); do
    start_time=$(date +%s)
    response=$(curl -s -o /dev/null -w '%{http_code}' $api_endpoint)
    end_time=$(date +%s)
    response_time=$(echo "$end_time - start_time" | bc)
    echo "iteration $i: response Time = ${response_time}s, status code = $response" >>"$api_response_file"
  done
  echo "api response test completed. results saved in $api_response_file."

  #------------------------------------------------------------------------
}

echo "$json" | jq -c '.[].address' | while IFS= read -r address; do
  echo "$json" | jq -c '.[].identity_key' | while IFS= read -r identity_key; do
    identity_key=$(echo "$identity_key" | jq -r .)
    exit_address=$(echo "$address" | jq -r .)

    sudo ./nym-vpn-cli -c sandbox.env --entry-gateway-id "$identity_key" --exit-router-address "$exit_address" --enable-two-hop >"$temp_log_file" 2>&1 &

    timeout=20
    start_time=$(date +%s)
    while true; do
      if grep -q "received plain" "$temp_log_file"; then
        echo "successful configuration with identity_key: $identity_key and exit address: $exit_address" >>two_hop_perf_test_results.log
        perform_tests "$identity_key" "$exit_address"
        break
      fi

      current_time=$(date +%s)
      if ((current_time - start_time > timeout)); then
        echo "failed to connect with identity_key: $identity_key using the exit address: $exit_address" >>two_hop_perf_test_results.log
        break
      fi

      sleep 1
    done
    cleanup
  done
done

rm -f "$temp_log_file"

```
