import { Network } from '../types';

export type NetworkSwitchOutcome =
  | { status: 'unchanged'; network: Network }
  | { status: 'committed'; network: Network }
  | { status: 'failed'; network: Network };

/**
 * `loadAccount` swallows backend errors and resolves to `undefined`, so the absence of a thrown
 * error is NOT proof the Rust `switch_network` succeeded. A switch succeeded only when a client
 * was actually loaded for the target network.
 */
export function didNetworkRefreshSucceed(loadedClient: unknown): boolean {
  return Boolean(loadedClient);
}

/** Logged-in switches commit network state only after Rust refresh succeeds. */
export function resolveNetworkSwitchOutcome(
  previousNetwork: Network | undefined,
  targetNetwork: Network,
  refreshSucceeded: boolean,
  hasActiveSession: boolean,
): NetworkSwitchOutcome {
  if (targetNetwork === previousNetwork) {
    return { status: 'unchanged', network: previousNetwork ?? targetNetwork };
  }
  if (!hasActiveSession) {
    return { status: 'committed', network: targetNetwork };
  }
  if (refreshSucceeded) {
    return { status: 'committed', network: targetNetwork };
  }
  return { status: 'failed', network: previousNetwork ?? targetNetwork };
}

/** React setState is async; pass the committed network when persisting immediately after a switch. */
export function selectNetworkForPersistence(
  reactStateNetwork: Network | undefined,
  explicitNetwork?: Network,
): Network | undefined {
  return explicitNetwork ?? reactStateNetwork;
}
