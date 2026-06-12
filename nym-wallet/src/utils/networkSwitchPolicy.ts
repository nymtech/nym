/** Logged-in network changes must call Rust `switch_network`; the boot effect only loads when no session yet. */
export function shouldRefreshAccountOnManualNetworkSwitch(hasActiveSession: boolean): boolean {
  return hasActiveSession;
}
