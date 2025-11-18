#!/bin/bash
# helper script to verify firewall ordering
set -euo pipefail

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
warn() { printf '\033[31m⚠ %s\033[0m\n' "$*"; }
ok() { printf '\033[32m✓ %s\033[0m\n' "$*"; }

RULE_OFFSET=2 # this is because thefirst rule appears on line 3

get_rule_line() {
  local chain=$1
  local rule_idx=$2
  iptables -L "$chain" -n --line-numbers | sed -n "$((rule_idx + RULE_OFFSET))p"
}

check_forward_chain() {
  local output
  output=$(iptables -L FORWARD -n --line-numbers)

  if echo "$output" | grep -q "^1[[:space:]]\+NYM-EXIT"; then
    ok "FORWARD rule 1 jumps to NYM-EXIT"
  else
    warn "FORWARD rule 1 is not NYM-EXIT; re-run network-tunnel-manager.sh exit_policy_install"
    return 1
  fi

  if echo "$output" | grep -q "ACCEPT.*state RELATED,ESTABLISHED"; then
    ok "FORWARD chain contains RELATED,ESTABLISHED accepts (WG return path)"
  else
    warn "FORWARD chain missing RELATED,ESTABLISHED accepts; re-run network-tunnel-manager.sh apply_iptables_rules_wg"
    return 1
  fi

  return 0
}

check_nym_exit_chain() {
  local errors=0
  local patterns=(
    "udp.*dpt:53"
    "tcp.*dpt:53"
    "icmp.*type 8"
    "icmp.*type 0"
  )

  for idx in "${!patterns[@]}"; do
    local rule_no=$((idx + 1))
    local line
    line=$(get_rule_line "NYM-EXIT" "$rule_no")
    if [[ "$line" =~ ${patterns[$idx]} ]]; then
      ok "NYM-EXIT rule $rule_no matches ${patterns[$idx]}"
    else
      warn "NYM-EXIT rule $rule_no is not ${patterns[$idx]}; re-run network-tunnel-manager.sh exit_policy_install"
      errors=1
    fi
  done

  local last_rule
  last_rule=$(iptables -L NYM-EXIT -n --line-numbers | awk 'NR>2 {line=$0} END {print line}')
  if [[ -z "${last_rule:-}" ]]; then
    warn "NYM-EXIT chain is empty; re-run network-tunnel-manager.sh exit_policy_install"
    errors=1
  elif [[ "$last_rule" =~ REJECT ]] && [[ "$last_rule" =~ 0\.0\.0\.0/0 ]]; then
    ok "NYM-EXIT ends with REJECT all"
  else
    warn "NYM-EXIT final rule is not a REJECT all (got: $last_rule)"
    errors=1
  fi

  return $errors
}

main() {
  bold "Checking IPv4 firewall ordering…"
  local errors=0
  check_forward_chain || errors=1
  check_nym_exit_chain || errors=1

  if command -v ip6tables >/dev/null 2>&1; then
    bold "Checking IPv6 firewall ordering…"
    if ip6tables -L NYM-EXIT -n --line-numbers >/dev/null 2>&1; then
      if ! ip6tables -L NYM-EXIT -n --line-numbers | sed -n '3p' | grep -q "udp.*dpt:53"; then
        warn "ip6tables NYM-EXIT rule 1 is not UDP 53"
        errors=1
      fi
    fi
  fi

  if [[ $errors -ne 0 ]]; then
    warn "There may be some problems, it's recommended to re-run network-tunnel-manager.sh exit_policy_install after configuring UFW."
    exit 1
  else
    ok "It's looking good!"
  fi
}

main "$@"

