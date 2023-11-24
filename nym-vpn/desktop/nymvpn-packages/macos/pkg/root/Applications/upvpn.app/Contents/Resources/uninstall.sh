#!/usr/bin/env bash

set -ueo pipefail

LOG_DIR="/var/log/nymvpn"
NYMVPN_DAEMON_PLIST_PATH="/Library/LaunchDaemons/net.nymtech.daemon.plist"

ask_confirmation() {
    read -p "Are you sure you want to stop and uninstall nymvpn? (y/n) "
    if [[ "$REPLY" =~ [Yy]$ ]]; then
        echo "Uninstalling nymvpn ..."
    else
        echo "Thank you for keeping nymvpn"
        exit 0
    fi
}

stop_daemon_and_ui() {
    echo "Stopping and unloading nymvpn-daemon ..."
    if [[ -f "${NYMVPN_DAEMON_PLIST_PATH}" ]]; then
        # Stop Daemon and UI
        sudo launchctl unload -w "${NYMVPN_DAEMON_PLIST_PATH}" || true
        sudo pkill -x "nymvpn-ui" || true
        sudo rm "${NYMVPN_DAEMON_PLIST_PATH}" || true
    fi
}

uninstall() {
    echo "Removing files ..."
    sudo rm /usr/local/bin/nymvpn || true
    sudo rm -rf /Applications/nymvpn.net || true
    sudo pkgutil --forget net.nymtech.vpn || true
    sudo rm -rf /var/log/nymvpn || true
    sudo rm -rf /etc/nymvpn || true
}

main() {
    ask_confirmation
    stop_daemon_and_ui
    uninstall
    echo "Done."
}

main "$@"
