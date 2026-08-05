#!/bin/bash

if [[ "$(id -u)" -ne 0 ]]; then
  echo "This script must be run as root."
  exit 1
fi

# Non-interactive so nothing blocks on a prompt during automated setup
export DEBIAN_FRONTEND=noninteractive
set -euo pipefail

echo -e "\n* * * Installing needed prerequisites * * *"

# --- Recover from any previously interrupted dpkg/apt state ---
# A killed install or a reboot mid-install leaves dpkg half-configured and every
# subsequent apt call fails with: "dpkg was interrupted, you must manually run
# 'dpkg --configure -a'". Run the recovery unconditionally; it is a no-op when clean.
echo "Ensuring package system is in a consistent state..."
dpkg --configure -a || true
apt-get --fix-broken install -y || true

# --- Update and upgrade ---
apt-get update -y
apt-get upgrade -y

# --- Core dependencies (hard requirements) ---
# If any of these fail the node cannot be set up, so we let a failure surface.
apt-get install -y \
  ca-certificates jq curl wget ufw tmux pkg-config build-essential \
  libssl-dev git nginx

# --- Optional/convenience packages (best-effort) ---
# Package names differ across Debian 12, Ubuntu 22/24/26 (e.g. ntp -> ntpsec,
# ntpdate deprecated). Install each independently so a missing one on a given
# release does not abort the whole run.
for pkg in tree tig neovim; do
  apt-get install -y "$pkg" || echo "[WARN] optional package '$pkg' not installed (not available on this release)"
done

# --- Time sync (critical for WireGuard handshake validity) ---
# Try modern then legacy providers; whichever exists on this release wins.
if apt-get install -y ntpsec 2>/dev/null; then
  echo "[OK] time sync via ntpsec"
elif apt-get install -y ntp 2>/dev/null; then
  echo "[OK] time sync via ntp"
elif apt-get install -y systemd-timesyncd 2>/dev/null; then
  systemctl enable --now systemd-timesyncd 2>/dev/null || true
  echo "[OK] time sync via systemd-timesyncd"
else
  echo "[WARN] no NTP package could be installed; ensure clock is synced manually"
fi

echo -e "\n* * * Prerequisites installed * * *"
echo "Firewall (ufw) configuration is handled by the CLI according to node mode."