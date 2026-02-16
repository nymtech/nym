#!/bin/bash

# Nym Localnet Load Test
# Generates sustained traffic through the mixnet SOCKS5 proxy to produce
# OTel traces and exercise the packet pipeline end-to-end.
#
# Usage:
#   ./loadtest.sh                       # defaults: 10 concurrent, 60s, mixed sizes
#   ./loadtest.sh -c 20 -d 120         # 20 concurrent, 120s
#   ./loadtest.sh -s 64k               # fixed 64KB responses (many Sphinx fragments)
#   ./loadtest.sh -s 1k -c 5 -d 30    # small payloads, 5 workers
#
# Payload sizes (-s flag) map to Sphinx packet fragmentation:
#   1k   = ~1 Sphinx packet    (sub-MTU, minimal fragmentation)
#   4k   = ~2-3 packets        (small payload)
#   16k  = ~8-10 packets       (medium payload)
#   64k  = ~32-35 packets      (large payload, stresses forwarding)
#   256k = ~128-130 packets    (heavy payload, stresses queues)
#   1m   = ~512 packets        (very heavy, potential backpressure)
#
# Prerequisites:
#   - Localnet running (./localnet.sh start)
#   - SOCKS5 proxy available on localhost:1080

set -e

CONCURRENCY=10
DURATION=60
PROXY="socks5h://127.0.0.1:1080"
PAYLOAD_SIZE=""
CUSTOM_URL=""
STATS_INTERVAL=5

# Default targets: mixed sizes for general testing
TARGETS=(
    "https://httpbin.org/get"
    "https://httpbin.org/bytes/1024"
    "https://httpbin.org/delay/1"
    "https://example.com"
    "https://nym.com"
)

# Convert human-readable size to bytes for httpbin
parse_size() {
    local s
    s=$(echo "$1" | tr '[:upper:]' '[:lower:]')
    local num
    num=$(echo "$s" | sed 's/[a-z]*$//')
    case "$s" in
        *m|*mb) echo $(( num * 1024 * 1024 )) ;;
        *k|*kb) echo $(( num * 1024 )) ;;
        *)      echo "$num" ;;
    esac
}

usage() {
    echo "Usage: $0 [-c concurrency] [-d duration_secs] [-s payload_size] [-u url] [-p proxy]"
    echo ""
    echo "Options:"
    echo "  -c  Number of concurrent workers (default: $CONCURRENCY)"
    echo "  -d  Test duration in seconds (default: $DURATION)"
    echo "  -s  Response payload size: 1k, 4k, 16k, 64k, 256k, 1m (default: mixed)"
    echo "  -u  Custom target URL (overrides -s and default targets)"
    echo "  -p  SOCKS5 proxy address (default: $PROXY)"
    echo ""
    echo "Examples:"
    echo "  $0                         # 10 workers, 60s, mixed targets/sizes"
    echo "  $0 -s 1k                   # small payloads (~1 Sphinx packet each)"
    echo "  $0 -s 64k -c 5            # large payloads, 5 workers"
    echo "  $0 -s 256k -c 2 -d 30     # very large payloads, observe queue pressure"
    echo "  $0 -c 20 -d 120           # heavier concurrency, 2 minutes"
    exit 0
}

while getopts "c:d:s:u:p:h" opt; do
    case $opt in
        c) CONCURRENCY=$OPTARG ;;
        d) DURATION=$OPTARG ;;
        s) PAYLOAD_SIZE=$OPTARG ;;
        u) CUSTOM_URL=$OPTARG ;;
        p) PROXY=$OPTARG ;;
        h) usage ;;
        *) usage ;;
    esac
done

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_err()   { echo -e "${RED}[ERROR]${NC} $*"; }

# Build sized URL if -s was specified
SIZED_URL=""
SIZE_LABEL="mixed"
if [ -n "$PAYLOAD_SIZE" ]; then
    PAYLOAD_BYTES=$(parse_size "$PAYLOAD_SIZE")
    SIZED_URL="https://httpbin.org/bytes/${PAYLOAD_BYTES}"
    SIZE_LABEL="${PAYLOAD_SIZE} (~${PAYLOAD_BYTES} bytes)"
fi

# Preflight checks
if ! nc -z 127.0.0.1 1080 2>/dev/null; then
    log_err "SOCKS5 proxy not reachable on localhost:1080. Is the localnet running?"
    exit 1
fi

# Counters (written to temp files for cross-process aggregation)
STATS_DIR=$(mktemp -d)
cleanup() {
    kill $(jobs -p) 2>/dev/null || true
    rm -rf "$STATS_DIR"
}
trap cleanup INT TERM EXIT

pick_url() {
    if [ -n "$CUSTOM_URL" ]; then
        echo "$CUSTOM_URL"
    elif [ -n "$PAYLOAD_SIZE" ]; then
        echo "$SIZED_URL"
    else
        local idx=$((RANDOM % ${#TARGETS[@]}))
        echo "${TARGETS[$idx]}"
    fi
}

# Millisecond timestamp (works on both GNU and BSD/macOS date)
now_ms() {
    python3 -c 'import time; print(int(time.time()*1000))'
}

# Worker function: runs requests in a loop until duration expires
worker() {
    local id=$1
    local end_time=$2
    local ok=0
    local fail=0

    while [ "$(date +%s)" -lt "$end_time" ]; do
        local url
        url=$(pick_url)
        local start_ms
        start_ms=$(now_ms)

        if curl -x "$PROXY" -m 15 -sf -o /dev/null -w "" "$url" 2>/dev/null; then
            ok=$((ok + 1))
        else
            fail=$((fail + 1))
        fi

        local end_ms
        end_ms=$(now_ms)
        local latency=$((end_ms - start_ms))

        echo "$latency" >> "$STATS_DIR/latencies_${id}.txt"
    done

    echo "$ok" > "$STATS_DIR/ok_${id}.txt"
    echo "$fail" > "$STATS_DIR/fail_${id}.txt"
}

echo ""
log_info "=== Nym Localnet Load Test ==="
log_info "Concurrency: $CONCURRENCY workers"
log_info "Duration:    ${DURATION}s"
log_info "Payload:     $SIZE_LABEL"
if [ -n "$CUSTOM_URL" ]; then
    log_info "Target:      $CUSTOM_URL"
elif [ -n "$PAYLOAD_SIZE" ]; then
    log_info "Target:      $SIZED_URL"
else
    log_info "Targets:     ${#TARGETS[@]} rotating URLs"
fi
log_info "Proxy:       $PROXY"
echo ""

# Quick connectivity check
log_info "Preflight: testing SOCKS5 proxy..."
if curl -x "$PROXY" -m 15 -sf -o /dev/null "https://httpbin.org/get"; then
    log_ok "SOCKS5 proxy is working"
else
    log_err "SOCKS5 proxy test failed. Check localnet status."
    exit 1
fi

END_TIME=$(( $(date +%s) + DURATION ))
START_TIME=$(date +%s)

log_info "Starting $CONCURRENCY workers for ${DURATION}s..."
echo ""

for i in $(seq 1 "$CONCURRENCY"); do
    worker "$i" "$END_TIME" &
done

# Progress reporter (counts completed latency entries as a proxy for request count)
while [ "$(date +%s)" -lt "$END_TIME" ]; do
    sleep "$STATS_INTERVAL"
    elapsed=$(( $(date +%s) - START_TIME ))
    remaining=$(( END_TIME - $(date +%s) ))
    if [ "$remaining" -lt 0 ]; then remaining=0; fi

    total=0
    for f in "$STATS_DIR"/latencies_*.txt; do
        if [ -f "$f" ]; then
            count=$(wc -l < "$f" 2>/dev/null || echo 0)
            total=$((total + count))
        fi
    done

    if [ "$elapsed" -gt 0 ]; then
        rps=$(echo "scale=1; $total / $elapsed" | bc 2>/dev/null || echo "?")
    else
        rps="?"
    fi

    printf "\r  [%3ds / %3ds]  requests: %d  |  ~%s req/s  |  remaining: %ds  " \
        "$elapsed" "$DURATION" "$total" "$rps" "$remaining"
done

echo ""
log_info "Waiting for workers to finish..."
wait 2>/dev/null || true

# Final stats
echo ""
log_info "=== Results ==="
total_ok=0
total_fail=0
all_latencies=""

for f in "$STATS_DIR"/ok_*.txt; do
    [ -f "$f" ] && total_ok=$((total_ok + $(cat "$f" 2>/dev/null || echo 0)))
done
for f in "$STATS_DIR"/fail_*.txt; do
    [ -f "$f" ] && total_fail=$((total_fail + $(cat "$f" 2>/dev/null || echo 0)))
done
for f in "$STATS_DIR"/latencies_*.txt; do
    [ -f "$f" ] && all_latencies="$all_latencies $(cat "$f" 2>/dev/null | tr '\n' ' ')"
done

total=$((total_ok + total_fail))
actual_duration=$(( $(date +%s) - START_TIME ))

echo ""
echo "  Total requests:   $total"
echo "  Successful:       $total_ok"
echo "  Failed:           $total_fail"
if [ "$actual_duration" -gt 0 ]; then
    rps=$(echo "scale=2; $total / $actual_duration" | bc 2>/dev/null || echo "?")
    echo "  Duration:         ${actual_duration}s"
    echo "  Throughput:       ~${rps} req/s"
fi

if [ -n "$all_latencies" ]; then
    sorted=$(echo "$all_latencies" | tr ' ' '\n' | sort -n | grep -v '^$')
    count=$(echo "$sorted" | wc -l | tr -d ' ')
    if [ "$count" -gt 0 ]; then
        p50_idx=$(( count * 50 / 100 ))
        p95_idx=$(( count * 95 / 100 ))
        p99_idx=$(( count * 99 / 100 ))
        [ "$p50_idx" -lt 1 ] && p50_idx=1
        [ "$p95_idx" -lt 1 ] && p95_idx=1
        [ "$p99_idx" -lt 1 ] && p99_idx=1

        min_lat=$(echo "$sorted" | head -1)
        max_lat=$(echo "$sorted" | tail -1)
        p50=$(echo "$sorted" | sed -n "${p50_idx}p")
        p95=$(echo "$sorted" | sed -n "${p95_idx}p")
        p99=$(echo "$sorted" | sed -n "${p99_idx}p")

        echo ""
        echo "  Latency (ms):"
        echo "    min:  ${min_lat}ms"
        echo "    p50:  ${p50}ms"
        echo "    p95:  ${p95}ms"
        echo "    p99:  ${p99}ms"
        echo "    max:  ${max_lat}ms"
    fi
fi

echo ""
if [ "$total_fail" -gt 0 ] && [ "$total" -gt 0 ]; then
    fail_pct=$(echo "scale=1; $total_fail * 100 / $total" | bc 2>/dev/null || echo "?")
    log_warn "Failure rate: ${fail_pct}% -- ${total_fail} of ${total} failed"
else
    log_ok "All requests succeeded"
fi
echo ""
log_info "View traces in SigNoz: http://localhost:8080/traces"
log_info "Filter by service: nym-node"
echo ""
