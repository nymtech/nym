/* eslint-disable @typescript-eslint/naming-convention */
import React, { useCallback, useMemo, useRef, useState } from 'react';
import { TransactionExecuteResult } from '@nymproject/types';
import { useQueryClient } from '@tanstack/react-query';
import { FamiliesContext, FamilyExecutingAction, FamilyQueries, TFamiliesContext } from 'src/context/families';
import { familyQueryKeys } from 'src/context/familyQueryKeys';
import { FamilyEvent, FamilyTxResult, NodeId } from 'src/types/families';
import { useNowSecs } from 'src/hooks/useNowSecs';
import { mockSleep } from './utils';
import { buildSeededStore, MOCK_OWNER_ADDRESS } from './families.fixtures';
import {
  MockStore,
  mockAcceptFamilyInvitation,
  mockCreateFamily,
  mockDisbandFamily,
  mockGetFamilyById,
  mockGetFamilyByOwner,
  mockGetFamilyConfig as getConfig,
  mockGetFamilyMembersPaged,
  mockGetFamilyMembership,
  mockGetPastInvitationsForFamilyPaged,
  mockGetPastMembersForFamilyPaged,
  mockGetPendingInvitationsForFamilyPaged,
  mockGetPendingInvitationsForNodePaged,
  mockInviteToFamily,
  mockKickFromFamily,
  mockLeaveFamily,
  mockRejectFamilyInvitation,
  mockRevokeFamilyInvitation,
  mockUpdateFamily,
} from './familiesMockState';

const TxResultMock: TransactionExecuteResult = {
  logs_json: '',
  msg_responses_json: '',
  transaction_hash: '55303CD4B91FAC4C2715E40EBB52BB3B92829D9431B3A279D37B5CC58432E354',
  gas_info: {
    gas_wanted: { gas_units: BigInt(1) },
    gas_used: { gas_units: BigInt(1) },
  },
  fee: { amount: '1', denom: 'nym' },
};

const buildTxResult = (family_events: FamilyEvent[]): FamilyTxResult => ({ ...TxResultMock, family_events });

const controlledFor = (store: MockStore, sender: string): NodeId[] =>
  [...store.bondedNodes.entries()].filter(([, b]) => b.owner === sender && !b.isUnbonding).map(([nodeId]) => nodeId);

export interface MockFamiliesProviderProps {
  /** Pre-built store; defaults to the richly-seeded fixture store. */
  store?: MockStore;
  /** Connected wallet address (the persona). */
  sender?: string;
  /** Simulated IPC latency in ms. */
  latencyMs?: number;
  children?: React.ReactNode;
}

export const MockFamiliesContextProvider = ({
  store: storeProp,
  sender = MOCK_OWNER_ADDRESS,
  latencyMs = 400,
  children,
}: MockFamiliesProviderProps): React.JSX.Element => {
  const queryClient = useQueryClient();
  const storeRef = useRef<MockStore>(storeProp ?? buildSeededStore());
  const [executingAction, setExecutingAction] = useState<FamilyExecutingAction>(null);
  const [error, setError] = useState<string>();
  const nowSecs = useNowSecs();

  const controlledNodeIds = useMemo(() => controlledFor(storeRef.current, sender), [sender]);

  const refreshAll = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: familyQueryKeys.all });
  }, [queryClient]);

  const clearError = useCallback(() => setError(undefined), []);

  const queries = useMemo<FamilyQueries>(
    () => ({
      getFamilyConfig: async () => {
        await mockSleep(latencyMs);
        return getConfig(storeRef.current);
      },
      getFamilyById: async (familyId) => {
        await mockSleep(latencyMs);
        return mockGetFamilyById(storeRef.current, familyId);
      },
      getFamilyByOwner: async (owner) => {
        await mockSleep(latencyMs);
        return mockGetFamilyByOwner(storeRef.current, owner);
      },
      getFamilyMembership: async (nodeId) => {
        await mockSleep(latencyMs);
        return mockGetFamilyMembership(storeRef.current, nodeId);
      },
      getFamilyMembersPaged: async (familyId, startAfter, limit) => {
        await mockSleep(latencyMs);
        return mockGetFamilyMembersPaged(storeRef.current, familyId, startAfter, limit);
      },
      getPendingInvitationsForFamilyPaged: async (familyId, startAfter, limit) => {
        await mockSleep(latencyMs);
        return mockGetPendingInvitationsForFamilyPaged(storeRef.current, familyId, startAfter, limit);
      },
      getPendingInvitationsForNodePaged: async (nodeId, startAfter, limit) => {
        await mockSleep(latencyMs);
        return mockGetPendingInvitationsForNodePaged(storeRef.current, nodeId, startAfter, limit);
      },
      getPastInvitationsForFamilyPaged: async (familyId, startAfter, limit) => {
        await mockSleep(latencyMs);
        return mockGetPastInvitationsForFamilyPaged(storeRef.current, familyId, startAfter, limit);
      },
      getPastMembersForFamilyPaged: async (familyId, startAfter, limit) => {
        await mockSleep(latencyMs);
        return mockGetPastMembersForFamilyPaged(storeRef.current, familyId, startAfter, limit);
      },
    }),
    [latencyMs],
  );

  const run = useCallback(
    async (
      action: NonNullable<FamilyExecutingAction>,
      mutate: (store: MockStore) => FamilyEvent[],
    ): Promise<FamilyTxResult> => {
      setExecutingAction(action);
      setError(undefined);
      await mockSleep(latencyMs);
      try {
        const events = mutate(storeRef.current);
        await refreshAll();
        return buildTxResult(events);
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        setError(message);
        throw e;
      } finally {
        setExecutingAction(null);
      }
    },
    [latencyMs, refreshAll],
  );

  const isExecuting = executingAction !== null;

  const value = useMemo<TFamiliesContext>(
    () => ({
      ownerAddress: sender,
      controlledNodeIds,
      nowSecs,
      queries,
      isExecuting,
      executingAction,
      error,
      clearError,
      refreshAll,
      createFamily: (args) => run('create', (s) => mockCreateFamily(s, sender, args)),
      updateFamily: (args) => run('update', (s) => mockUpdateFamily(s, sender, args)),
      disbandFamily: () => run('disband', (s) => mockDisbandFamily(s, sender)),
      inviteToFamily: (args) => run('invite', (s) => mockInviteToFamily(s, sender, args)),
      revokeFamilyInvitation: (args) => run('revoke', (s) => mockRevokeFamilyInvitation(s, sender, args)),
      kickFromFamily: (args) => run('kick', (s) => mockKickFromFamily(s, sender, args)),
      acceptFamilyInvitation: (args) => run('accept', (s) => mockAcceptFamilyInvitation(s, sender, args)),
      rejectFamilyInvitation: (args) => run('reject', (s) => mockRejectFamilyInvitation(s, sender, args)),
      leaveFamily: (args) => run('leave', (s) => mockLeaveFamily(s, sender, args)),
    }),
    [sender, controlledNodeIds, nowSecs, queries, isExecuting, executingAction, error, clearError, refreshAll, run],
  );

  return <FamiliesContext.Provider value={value}>{children}</FamiliesContext.Provider>;
};
