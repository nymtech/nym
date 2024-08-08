#!/bin/bash

# Very basic validation, only that endpoints are reachable
clear

source $(dirname "$0")/definitions.sh

function fire_on_all_apis() {
  local target_address="$1"

  circulating_supply

  contract_cache

  network

  api_status

  unstable
}

fire_on_all_apis "http://localhost:8081"

fire_on_all_apis "http://localhost:8000"
