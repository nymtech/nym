/* eslint-disable @typescript-eslint/naming-convention */
import React, { useCallback, useContext, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import * as familyRequests from 'src/requests/families';
import { Console } from 'src/utils/console';
import { FamilyTxResult, NodeId } from 'src/types/families';
import { isMixnode, isNymNode } from 'src/types/global';
import { useNowSecs } from 'src/hooks/useNowSecs';
import { AppContext } from './main';
import { FamiliesContext, FamilyExecutingAction, TFamiliesContext, defaultQueries } from './families';
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

  const [executingAction, setExecutingAction] = useState<FamilyExecutingAction>(null);
  const [error, setError] = useState<string>();
  /** Bumped on `clearError()` so late async handlers cannot re-surface a dismissed error. */
  const operationEpochRef = useRef(0);

  // The operator persona is "nodes I control". An account bonds at most one node,
  // so this is the bonded node's id (the unified mixnet node id: `nodeId` for a
  // nym-node, `mixId` for a legacy mixnode), or none for a gateway / no bond.
  // Sourced from the `BondingContext` the families route now wraps.
  const { bondedNode } = useBondingContext();
  const controlledNodeIds = useMemo<NodeId[]>(() => {
    if (!bondedNode) return [];
    if (isNymNode(bondedNode)) return [bondedNode.nodeId];
    if (isMixnode(bondedNode)) return [bondedNode.mixId];
    return [];
  }, [bondedNode]);

  const nowSecs = useNowSecs();

  const refreshAll = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: familyQueryKeys.all });
  }, [queryClient]);

  const clearError = useCallback(() => {
    operationEpochRef.current += 1;
    setError(undefined);
    setExecutingAction(null);
  }, []);

  /** Run an execute call: toggle flag, surface + rethrow errors, refresh reads on success. */
  const run = useCallback(
    async (action: NonNullable<FamilyExecutingAction>, op: () => Promise<FamilyTxResult>): Promise<FamilyTxResult> => {
      const epoch = operationEpochRef.current;
      setExecutingAction(action);
      setError(undefined);
      try {
        const result = await op();
        if (epoch !== operationEpochRef.current) {
          return result;
        }
        await refreshAll();
        return result;
      } catch (e) {
        if (epoch === operationEpochRef.current) {
          const message = e instanceof Error ? e.message : String(e);
          setError(message);
        }
        Console.error(e);
        throw e;
      } finally {
        if (epoch === operationEpochRef.current) {
          setExecutingAction(null);
        }
      }
    },
    [refreshAll],
  );

  const isExecuting = executingAction !== null;

  const memoizedValue = useMemo<TFamiliesContext>(
    () => ({
      ownerAddress,
      controlledNodeIds,
      nowSecs,
      queries: defaultQueries,
      isExecuting,
      executingAction,
      error,
      clearError,
      refreshAll,
      createFamily: (args) => run('create', () => familyRequests.createFamily(args)),
      updateFamily: (args) => run('update', () => familyRequests.updateFamily(args)),
      disbandFamily: () => run('disband', () => familyRequests.disbandFamily()),
      inviteToFamily: (args) => run('invite', () => familyRequests.inviteToFamily(args)),
      revokeFamilyInvitation: (args) => run('revoke', () => familyRequests.revokeFamilyInvitation(args)),
      kickFromFamily: (args) => run('kick', () => familyRequests.kickFromFamily(args)),
      acceptFamilyInvitation: (args) => run('accept', () => familyRequests.acceptFamilyInvitation(args)),
      rejectFamilyInvitation: (args) => run('reject', () => familyRequests.rejectFamilyInvitation(args)),
      leaveFamily: (args) => run('leave', () => familyRequests.leaveFamily(args)),
    }),
    [ownerAddress, controlledNodeIds, nowSecs, isExecuting, executingAction, error, clearError, refreshAll, run],
  );

  return <FamiliesContext.Provider value={memoizedValue}>{children}</FamiliesContext.Provider>;
};
