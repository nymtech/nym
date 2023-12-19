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

This version is an experimental software for testing purposes in a limited environment. This testing round aims to help Nym with:

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

We have CLI and UI binaries available for Linux (Debian based) and Mac. 

![](images/image1.png)

ADD DWL PAGE LINK

* Visit the [release page]() to download the binary for your system.
* Open terminal in the same directory and make executable by running `chmod +x ./nym-vpn_0.0.0_amd64.AppImage`

### CLI

### UI

## Running

***For NymVPN to work, all existing VPNs must be switched off!***

### CLI

Running a help command

```sh
./nym-vpn-client/target/release/nym-vpn-cli --help
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

USE SCRIPTS FPOR TESTING, DOCUMENT HOW AND HOW TO SHARE RESULTS

```sh
#!/bin/bash

api_key="YzI4Yjc2ZTI5NW1zaGIyYzZhYjNlNjM1ZmNkY3AxYTExODFqc25mM2FiNjBmODIxMGE="
ENV_URL="https://github.com/nymtech/nym/blob/develop/envs/sandbox.env"

if [ ! -f "sandbox.env" ]; then
    echo "sandbox.env not found. downloading..."
    curl -L "$ENV_URL" -o sandbox.env
else
    echo "sandbox.env already exists. Skipping download."
fi

echo "select the mode of operation:"
echo "1. decide between two-hop and wireguard"
echo "2. normal"
echo "3. manual input"
read -p "enter your choice (1/2/3): " mode_choice

echo "🚀 🏎 - please be patient, i'm fetching you your entry points - 🚀 🏎 "

cleanup() {
    echo "terminating nym-vpn-cli..."
    pkill -f './nym-vpn-cli'
    exit
}

get_lat_lon() {
    local ip=$1
    local response=$(curl -s "http://ip-api.com/json/${ip}")
    local lat=$(echo $response | jq '.lat')
    local lon=$(echo $response | jq '.lon')
    echo "$lat $lon"
}

calculate_distance() {
    local start_lat=$1
    local start_lon=$2
    local end_lat=$3
    local end_lon=$4

    local response=$(curl --silent --request GET \
        --url "https://distance-calculator8.p.rapidapi.com/calc?startLatitude=${start_lat}&startLongitude=${start_lon}&endLatitude=${end_lat}&endLongitude=${end_lon}" \
        --header "X-RapidAPI-Host: distance-calculator8.p.rapidapi.com" \
        --header "X-RapidAPI-Key: $(echo -n ${api_key} | base64 -d)")
    
    local distance_raw=$(echo $response | jq '.body.distance.kilometers')
    local distance=$(printf "%.0f" $distance_raw)
    echo $distance
}

trap cleanup SIGINT SIGTERM

ENDPOINT="https://sandbox-nym-api1.nymtech.net/api/v1/gateways/described"
MY_IP=$(curl -s http://ipecho.net/plain)

json_array=()

data=$(curl -s "$ENDPOINT" | jq -c '.[] | {host: .bond.gateway.host, identity_key: .bond.gateway.identity_key}')

while IFS= read -r entry; do
    host=$(echo "$entry" | jq -r '.host')
    identity_key=$(echo "$entry" | jq -r '.identity_key')
    response=$(curl -s "${host}:8080/api/v1/ip-packet-router")

    if [ -n "$response" ]; then
        full_address=$(echo "$response" | jq -r '.address')
        valid_ip=$(echo "$host")

        if [[ $valid_ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            country_info=$(curl -s "http://ipinfo.io/${valid_ip}/country" | tr -d '\n')
            country_info_escaped=$(echo "$country_info" | tr -d '\n' | jq -aRs . | tr -d '"')

            read MY_LAT MY_LON <<< $(get_lat_lon $MY_IP)
            read HOST_LAT HOST_LON <<< $(get_lat_lon $valid_ip)

            distance_to_gateway=$(calculate_distance $MY_LAT $MY_LON $HOST_LAT $HOST_LON)

            json_object="{\"host\": \"${host}\", \"identity_key\": \"${identity_key}\", \"address\": \"${full_address}\", \"country\": \"${country_info_escaped}\", \"distance_to_entry_gateway\": \"${distance_to_gateway}km\"}"
            json_array+=("$json_object")
        else
            country_info_escaped="invalid ip"
        fi
    else
        continue
    fi
done < <(echo "$data")

printf "%s\n" "${json_array[@]}" | jq -s . > temp.json
json_array_string=$(<temp.json)


addresses=($(jq -r '.[].address' <<< "$json_array_string"))
identity_keys=($(jq -r '.[].identity_key' <<< "$json_array_string"))

if [[ ${#addresses[@]} -eq 0 ]]; then
    echo "no addresses found, exiting."
    exit 1
fi

while : ; do
    random_index=$((RANDOM % ${#addresses[@]}))
    random_identity_key=${identity_keys[random_index]}
    random_address=${addresses[random_index]}

    exists=$(jq --arg ik "$random_identity_key" --arg ad "$random_address" -n '[inputs | select(.identity_key == $ik and .address == $ad)] | length' <<< "$json_array_string" 2>/dev/null)
    if [[ $exists -eq 0 ]]; then
        break 
    fi
done

case $mode_choice in
    1)
        read -p "do you want to enable WireGuard? enter (y/n): " answer 
        if [[ $answer == "y" ]]; then
            two_hop="--enable-two-hop --enable-wireguard --private-key ILeN6gEh6vJ3Ju8RJ3HVswz+sPgkcKtAYTqzQRhTtlo=" 
        fi
        two_hop="--enable-two-hop"
        ;;
    2)
        two_hop=""
        ;;
    3)
        printf "%s\n" "${json_array[@]}" | jq -s .
        read -p "enter your identity_key: " random_identity_key
        read -p "enter your exit_address: " random_address
        read -p "do you want to enter two-hop mode? enter (y/n): " answer

        if [[ $answer == "y" ]]; then
            read -p "do you want to enable WireGuard? enter (y/n): " wg_answer
            two_hop="--enable-two-hop"
            if [[ $wg_answer == "y" ]]; then
                two_hop="$two_hop --enable-wireguard --private-key ILeN6gEh6vJ3Ju8RJ3HVswz+sPgkcKtAYTqzQRhTtlo="
            fi
        else
            two_hop=""
        fi
        ;;
    *)
        echo "invalid choice, exiting."
        exit 1
        ;;
esac

echo "starting nym-vpn-cli"
echo "using configuration id_key: ${random_identity_key} :: exit_address: ${random_address}"
./nym-vpn-cli -c sandbox.env --entry-gateway "$random_identity_key" --exit-address "$random_address" $two_hop &

wait

cleanup

```
