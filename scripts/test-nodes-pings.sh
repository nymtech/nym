#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-https://validator.nymtech.net/api/v1/nym-nodes/described}"
CONCURRENCY="${CONCURRENCY:-64}"     # how many pings in flight
PING_TIMEOUT="${PING_TIMEOUT:-7}"    # seconds to wait for a single echo reply
PING_RETRIES="${PING_RETRIES:-1}"    # additional attempts after the first failure
RETRY_DELAY="${RETRY_DELAY:-1}"      # seconds to wait between attempts

OK_CSV="ping_works.csv"
BAD_CSV="ping_not_working.csv"

# check deps for sanity
need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing '$1'"; exit 1; }; }
need curl
need jq
need xargs
need nl
need ping

# fetch /described to json
echo "Fetching gateways description from: ${API_URL}"
tmp_json="$(mktemp)"
trap 'rm -f "$tmp_json" "$ip_list" "$num_list"' EXIT

curl -fsSL --retry 3 --retry-delay 1 --compressed "$API_URL" -o "$tmp_json"

# extract IPs
ip_list="$(mktemp)"
jq -r '
  .data[]?
  | (.description.host_information.ip_address? // [])[]
' "$tmp_json" \
| awk '
  # very permissive IPv4/IPv6 syntax filters (we let ping validate the rest)
  function is_ipv4(s){ return (s ~ /^[0-9]{1,3}(\.[0-9]{1,3}){3}$/) }
  function is_ipv6(s){ return (index(s,":") > 0) }
  { if(is_ipv4($0) || is_ipv6($0)) print $0 }
' \
| sort -u > "$ip_list"

TOTAL="$(wc -l < "$ip_list" | tr -d '[:space:]')"
if [[ "$TOTAL" -eq 0 ]]; then
  echo "No IP addresses found in API response. Exiting."
  exit 1
fi
echo "Collected ${TOTAL} unique IP addresses to probe."

# outputs
printf "ip\n" > "$OK_CSV"
printf "ip\n" > "$BAD_CSV"

num_list="$(mktemp)"
nl -ba "$ip_list" > "$num_list"

# probe function executed in parallel via xargs
export PING_TIMEOUT OK_CSV BAD_CSV TOTAL PING_RETRIES RETRY_DELAY

probe_one() {
  IFS=',' read -r idx ip <<< "$1"

  # returns 0 = success, 1 = failure after retries
  do_ping_with_retry() {
    local ip="$1"
    local attempts=$((1 + PING_RETRIES))
    local i
    for ((i=1; i<=attempts; i++)); do
      if [[ "$ip" == *:* ]]; then
        ping -6 -c 1 -W "$PING_TIMEOUT" "$ip" >/dev/null 2>&1 && return 0
      else
        ping    -c 1 -W "$PING_TIMEOUT" "$ip" >/dev/null 2>&1 && return 0
      fi
      # wait before next try if not the last attempt
      (( i < attempts )) && sleep "$RETRY_DELAY"
    done
    return 1
  }

  if do_ping_with_retry "$ip"; then
    printf "%s\n" "$ip" >> "$OK_CSV"
    printf "[%s/%s] ping ok %s\n" "$idx" "$TOTAL" "$ip"
  else
    printf "%s\n" "$ip" >> "$BAD_CSV"
    printf "[%s/%s] ping failed %s\n" "$idx" "$TOTAL" "$ip"
  fi
}

export -f probe_one

awk '{printf "%s,%s\n",$1,$2}' "$num_list" \
| xargs -P "$CONCURRENCY" -n 1 -I{} bash -c 'probe_one "$@"' _ {}

echo "Done. Results:"
echo "  $(($(wc -l < "$OK_CSV") - 1)) reachable -> $OK_CSV"
echo "  $(($(wc -l < "$BAD_CSV") - 1)) not reachable -> $BAD_CSV"

