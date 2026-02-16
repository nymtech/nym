#!/bin/bash

# Nym Localnet OTel Report
# Queries ClickHouse directly to produce a terminal-based summary of
# the core metrics captured by the OTel-instrumented nym-nodes.
#
# Usage:
#   ./otel-report.sh              # last 15 minutes
#   ./otel-report.sh 60           # last 60 minutes
#   ./otel-report.sh live         # live mode: refresh every 10s
#
# Prerequisites: localnet + SigNoz running

set -e

CH_CONTAINER="signoz-clickhouse"
TRACES_TABLE="signoz_traces.distributed_signoz_index_v3"
LOOKBACK_MIN=${1:-15}
LIVE=false

if [ "$1" = "live" ]; then
    LIVE=true
    LOOKBACK_MIN=5
fi

BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

ch() {
    docker exec "$CH_CONTAINER" clickhouse-client --query "$1" 2>/dev/null
}

divider() {
    echo -e "${DIM}$(printf '%.0s-' {1..78})${NC}"
}

print_report() {
    local window="$1"

    echo ""
    echo -e "${BOLD}  Nym Localnet -- OTel Packet Pipeline Report${NC}"
    echo -e "  ${DIM}Window: last ${window} minutes | $(date '+%Y-%m-%d %H:%M:%S')${NC}"
    divider

    # 1. Throughput per operation
    echo -e "\n${BOLD}  [1] Packet Throughput (packets/sec by operation)${NC}\n"
    ch "
    SELECT
        name AS operation,
        count(*) AS total,
        round(count(*) / (${window} * 60), 1) AS per_sec
    FROM ${TRACES_TABLE}
    WHERE timestamp >= now() - INTERVAL ${window} MINUTE
      AND serviceName = 'nym-node'
      AND name IN (
        'handle_received_nym_packet',
        'mixnode.sphinx_full_unwrap',
        'mixnode.forward_packet',
        'mixnode.final_hop'
      )
    GROUP BY name
    ORDER BY total DESC
    FORMAT PrettyCompactNoEscapes
    "

    divider

    # 2. Latency per operation
    echo -e "\n${BOLD}  [2] Processing Latency (milliseconds)${NC}\n"
    ch "
    SELECT
        name AS operation,
        round(quantile(0.50)(duration_nano / 1e6), 3) AS p50_ms,
        round(quantile(0.95)(duration_nano / 1e6), 3) AS p95_ms,
        round(quantile(0.99)(duration_nano / 1e6), 3) AS p99_ms,
        round(quantile(0.999)(duration_nano / 1e6), 3) AS p999_ms,
        round(avg(duration_nano / 1e6), 3) AS avg_ms
    FROM ${TRACES_TABLE}
    WHERE timestamp >= now() - INTERVAL ${window} MINUTE
      AND serviceName = 'nym-node'
      AND name IN (
        'handle_received_nym_packet',
        'mixnode.sphinx_full_unwrap',
        'mixnode.forward_packet',
        'mixnode.final_hop'
      )
      AND duration_nano < 60000000000
    GROUP BY name
    ORDER BY p50_ms DESC
    FORMAT PrettyCompactNoEscapes
    "

    divider

    # 3. Error rate
    echo -e "\n${BOLD}  [3] Error Rate${NC}\n"
    local errors
    errors=$(ch "
    SELECT
        name,
        countIf(has_error = true) AS errors,
        count(*) AS total,
        round(100.0 * countIf(has_error = true) / count(*), 3) AS error_pct
    FROM ${TRACES_TABLE}
    WHERE timestamp >= now() - INTERVAL ${window} MINUTE
      AND serviceName = 'nym-node'
      AND name IN (
        'handle_received_nym_packet',
        'mixnode.sphinx_full_unwrap',
        'mixnode.forward_packet',
        'mixnode.final_hop'
      )
    GROUP BY name
    HAVING errors > 0
    ORDER BY errors DESC
    FORMAT PrettyCompactNoEscapes
    ")

    if [ -z "$errors" ]; then
        echo -e "  ${GREEN}No errors detected across all operations${NC}"
    else
        echo "$errors"
    fi

    divider

    # 4. Forwarding ratio (are packets being dropped between stages?)
    echo -e "\n${BOLD}  [4] Pipeline Funnel (packet drop detection)${NC}\n"
    ch "
    SELECT
        name AS stage,
        count(*) AS packets,
        round(100.0 * count(*) / max(total_ingress), 1) AS pct_of_ingress
    FROM ${TRACES_TABLE}
    CROSS JOIN (
        SELECT count(*) AS total_ingress
        FROM ${TRACES_TABLE}
        WHERE timestamp >= now() - INTERVAL ${window} MINUTE
          AND serviceName = 'nym-node'
          AND name = 'handle_received_nym_packet'
    ) AS t
    WHERE timestamp >= now() - INTERVAL ${window} MINUTE
      AND serviceName = 'nym-node'
      AND name IN (
        'handle_received_nym_packet',
        'mixnode.sphinx_full_unwrap',
        'mixnode.forward_packet',
        'mixnode.final_hop'
      )
    GROUP BY name
    ORDER BY packets DESC
    FORMAT PrettyCompactNoEscapes
    "

    echo ""
    echo -e "  ${DIM}Expected ratios: sphinx_unwrap ~ 100%, forward ~ 75% (3 of 4 hops forward),${NC}"
    echo -e "  ${DIM}final_hop ~ 25% (1 of 4 hops is the last one). Significantly lower = drops.${NC}"

    divider

    # 5. Throughput over time (1-minute buckets)
    echo -e "\n${BOLD}  [5] Throughput Timeline (1-min buckets, ingress packets)${NC}\n"
    ch "
    SELECT
        toStartOfMinute(timestamp) AS minute,
        count(*) AS packets,
        round(count(*) / 60, 1) AS per_sec
    FROM ${TRACES_TABLE}
    WHERE timestamp >= now() - INTERVAL ${window} MINUTE
      AND serviceName = 'nym-node'
      AND name = 'handle_received_nym_packet'
    GROUP BY minute
    ORDER BY minute
    FORMAT PrettyCompactNoEscapes
    "

    divider

    # 6. Latency spikes (potential TCP congestion / backpressure indicators)
    echo -e "\n${BOLD}  [6] Latency Spikes (sphinx_unwrap p99 per minute)${NC}\n"
    ch "
    SELECT
        toStartOfMinute(timestamp) AS minute,
        round(quantile(0.99)(duration_nano / 1e6), 3) AS p99_ms,
        round(quantile(0.50)(duration_nano / 1e6), 3) AS p50_ms,
        round(p99_ms / greatest(p50_ms, 0.001), 1) AS spike_ratio,
        count(*) AS samples
    FROM ${TRACES_TABLE}
    WHERE timestamp >= now() - INTERVAL ${window} MINUTE
      AND serviceName = 'nym-node'
      AND name = 'mixnode.sphinx_full_unwrap'
    GROUP BY minute
    ORDER BY minute
    FORMAT PrettyCompactNoEscapes
    "

    echo ""
    echo -e "  ${DIM}spike_ratio > 10x suggests backpressure or queue buildup.${NC}"
    echo -e "  ${DIM}Sustained high p99 across minutes may indicate TCP meltdown.${NC}"

    divider
    echo ""
    echo -e "  ${BLUE}SigNoz UI:${NC} http://localhost:8080"
    echo -e "  ${DIM}Traces tab -> Filter: serviceName = nym-node${NC}"
    echo ""
}

if [ "$LIVE" = "true" ]; then
    while true; do
        clear
        print_report "$LOOKBACK_MIN"
        echo -e "  ${DIM}Refreshing in 10s... (Ctrl+C to stop)${NC}"
        sleep 10
    done
else
    print_report "$LOOKBACK_MIN"
fi
