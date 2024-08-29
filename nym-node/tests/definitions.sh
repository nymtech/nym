#!/bin/bash

# color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
DEFAULT='\033[0m'

axum_server_addr="http://localhost:8081"
rocket_server_addr="http://localhost:8000"

function compare_responses() {
    local api="$1"
    shift
    local url_1="${axum_server_addr}${api}"
    local url_2="${rocket_server_addr}${api}"

    local response_1=$(curl -s "${url_1}")
    local normalized_resp_1=$(echo "${response_1}" | jq --sort-keys '.')
    local response_2=$(curl -s "${url_2}")
    local normalized_resp_2=$(echo "${response_2}" | jq --sort-keys '.')

    echo -e "Response: \n${response_1}"
    # jq . "${response_1}"

    if [[ "${response_1}" == "${response_2}" ]]; then
    # if cmp -s <(jq -S . "$response_1") <(jq -S ."$response_2"); then
        echo -e "${GREEN}Responses are the same ${DEFAULT}"
    else
        echo -e "${RED}"
        echo "${response_2}"
        # jq . "${response_2}"
        echo -e "Responses are different"
        diff <(echo "${normalized_resp_1}") <(echo "${normalized_resp_2}")
        echo -e "${DEFAULT}"
    fi
}

function validate_response() {
    local expected_status="$1"
    shift
    local url=("$@")
    local query=(curl -I -X 'GET' "${url[@]}" -H 'accept: application/json')
    # execute given curl
    echo -e "Executing\n${BLUE}\t${query[*]}${DEFAULT}"
    local response=$("${query[@]}" -w "\n%{http_code}\n%{content_type}")

    # parse status code & content type
    local http_code=$(echo "$response" | tail -n2 | head -n1)
    local content_type=$(echo "$response" | tail -n1)

    # -e flag for coloured output
    if [ "$http_code" -eq "$expected_status" ]; then
        echo -e "${GREEN}HTTP code $http_code as expected, Content-Type is $content_type ${DEFAULT}"
        return 0
    else
        echo -e "${RED}Unexpected HTTP code: $http_code, Content-Type is $content_type ${DEFAULT}"
        return 1
    fi
}

function title() {
    local title="$1"
    echo -e "\n${BLUE}\t╔═══ ${title} ═══╗${DEFAULT}"
}

function circulating_supply() {
    title "/circulating-supply"

    local api="/v1/circulating-supply"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/circulating-supply/total-supply-value"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/circulating-supply/circulating-supply-value"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}
}

function contract_cache() {
    title "/contract-cache"

    local api="/v1/mixnodes"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/mixnodes/detailed"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/gateways"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/mixnodes/active"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/mixnodes/active/detailed"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/mixnodes/rewarded"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/mixnodes/rewarded/detailed"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/mixnodes/blacklisted"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/gateways/blacklisted"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/epoch/reward_params"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/epoch/current"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}
}

function network() {
    title "/network"
    local api="/v1/network/details"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/network/nym-contracts"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/network/nym-contracts-detailed"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}
}

function api_status() {
    title "/api-status"
    local api="/v1/api-status/health"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/api-status/build-information"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/api-status/signer-information"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}
}

function unstable() {
    title "/unstable"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/skimmed"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/semi-skimmed"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/full-fat"

    local api="/v1/unstable/nym-nodes/gateways/skimmed"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/unstable/nym-nodes/gateways/skimmed?semver_compatibility"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/unstable/nym-nodes/gateways/skimmed?semver_compatibility=2.0.0"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/gateways/semi-skimmed"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/gateways/semi-skimmed?semver_compatibility"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/gateways/semi-skimmed?semver_compatibility=2.0.0"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/gateways/full-fat"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/gateways/full-fat?semver_compatibility"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/gateways/full-fat?semver_compatibility=2.0.0"

    local api="/v1/unstable/nym-nodes/mixnodes/skimmed"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/unstable/nym-nodes/mixnodes/skimmed?semver_compatibility"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    local api="/v1/unstable/nym-nodes/mixnodes/skimmed?semver_compatibility=2.0.0"
    validate_response 200 "${target_address}${api}"
    compare_responses ${api}

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/mixnodes/semi-skimmed"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/mixnodes/semi-skimmed?semver_compatibility"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/mixnodes/semi-skimmed?semver_compatibility=2.0.0"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/mixnodes/full-fat"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/mixnodes/full-fat?semver_compatibility"

    validate_response 501 "${target_address}/v1/unstable/nym-nodes/mixnodes/full-fat?semver_compatibility=2.0.0"
}
