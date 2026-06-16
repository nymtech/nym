/* eslint-disable @typescript-eslint/naming-convention */
import React, { useCallback, useContext, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import * as familyRequests from 'src/requests/families';
import { Console } from 'src/utils/console';
import { FamilyTxResult, NodeId } from 'src/types/families';
import { isMixnode, isNymNode } from 'src/types/global';
import { AppContext } from './main';
import { FamiliesContext, TFamiliesContext, defaultQueries } from './families';
import { useBondingContext } from './bonding';
import { familyQueryKeys } from './familyQueryKeys';

/**
 * Real, Tauri-backed FamiliesContext provider. Kept in its own module so it is the
 * ONLY families file importing `./main` (which pulls Tauri-runtime code at load).
 * Storybook/tests use `MockFamiliesContextProvider` instead and never load this.
 */
export const FamiliesContextProvider: FCWithChildren = ({ children }): React.JSX.Element => {
  const queryClient = useQueryClient();
  const { clientDetails } = useContext(AppContext);
  const ownerAddress = clientDetails?.client_address;

  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string>();

  // The operator persona is "nodes I control". An account bonds at most one node,
  // so this is the bonded node's id (the unified mixnet node id — `nodeId` for a
  // nym-node, `mixId` for a legacy mixnode), or none for a gateway / no bond
  // (design D3). Sourced from the `BondingContext` the families route now wraps.
  const { bondedNode } = useBondingContext();
  const controlledNodeIds = useMemo<NodeId[]>(() => {
    if (!bondedNode) return [];
    if (isNymNode(bondedNode)) return [bondedNode.nodeId];
    if (isMixnode(bondedNode)) return [bondedNode.mixId];
    return [];
  }, [bondedNode]);

  const nowSecs = useMemo(() => Math.floor(Date.now() / 1000), []);

  const refreshAll = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: familyQueryKeys.all });
  }, [queryClient]);

  const clearError = useCallback(() => setError(undefined), []);

  /** Run an execute call: toggle flag, surface + rethrow errors, refresh reads on success. */
  const run = useCallback(
    async (op: () => Promise<FamilyTxResult>): Promise<FamilyTxResult> => {
      setIsExecuting(true);
      setError(undefined);
      try {
        const result = await op();
        await refreshAll();
        return result;
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        setError(message);
        Console.error(e);
        throw e;
      } finally {
        setIsExecuting(false);
      }
    },
    [refreshAll],
  );

  const memoizedValue = useMemo<TFamiliesContext>(
    () => ({
      ownerAddress,
      controlledNodeIds,
      nowSecs,
      queries: defaultQueries,
      isExecuting,
      error,
      clearError,
      refreshAll,
      createFamily: (args) => run(() => familyRequests.createFamily(args)),
      updateFamily: (args) => run(() => familyRequests.updateFamily(args)),
      disbandFamily: () => run(() => familyRequests.disbandFamily()),
      inviteToFamily: (args) => run(() => familyRequests.inviteToFamily(args)),
      revokeFamilyInvitation: (args) => run(() => familyRequests.revokeFamilyInvitation(args)),
      kickFromFamily: (args) => run(() => familyRequests.kickFromFamily(args)),
      acceptFamilyInvitation: (args) => run(() => familyRequests.acceptFamilyInvitation(args)),
      rejectFamilyInvitation: (args) => run(() => familyRequests.rejectFamilyInvitation(args)),
      leaveFamily: (args) => run(() => familyRequests.leaveFamily(args)),
    }),
    [ownerAddress, controlledNodeIds, nowSecs, isExecuting, error, clearError, refreshAll, run],
  );

  return <FamiliesContext.Provider value={memoizedValue}>{children}</FamiliesContext.Provider>;
};
