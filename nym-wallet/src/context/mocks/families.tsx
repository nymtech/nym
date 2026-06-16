/* eslint-disable @typescript-eslint/naming-convention */
import React, { useCallback, useMemo, useRef, useState } from 'react';
import { TransactionExecuteResult } from '@nymproject/types';
import { useQueryClient } from '@tanstack/react-query';
import { FamiliesContext, FamilyQueries, TFamiliesContext } from 'src/context/families';
import { familyQueryKeys } from 'src/context/familyQueryKeys';
import { FamilyEvent, FamilyTxResult, NodeId } from 'src/types/families';
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
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string>();

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
    async (mutate: (store: MockStore) => FamilyEvent[]): Promise<FamilyTxResult> => {
      setIsExecuting(true);
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
        setIsExecuting(false);
      }
    },
    [latencyMs, refreshAll],
  );

  const value = useMemo<TFamiliesContext>(
    () => ({
      ownerAddress: sender,
      controlledNodeIds,
      nowSecs: storeRef.current.nowSecs,
      queries,
      isExecuting,
      error,
      clearError,
      refreshAll,
      createFamily: (args) => run((s) => mockCreateFamily(s, sender, args)),
      updateFamily: (args) => run((s) => mockUpdateFamily(s, sender, args)),
      disbandFamily: () => run((s) => mockDisbandFamily(s, sender)),
      inviteToFamily: (args) => run((s) => mockInviteToFamily(s, sender, args)),
      revokeFamilyInvitation: (args) => run((s) => mockRevokeFamilyInvitation(s, sender, args)),
      kickFromFamily: (args) => run((s) => mockKickFromFamily(s, sender, args)),
      acceptFamilyInvitation: (args) => run((s) => mockAcceptFamilyInvitation(s, sender, args)),
      rejectFamilyInvitation: (args) => run((s) => mockRejectFamilyInvitation(s, sender, args)),
      leaveFamily: (args) => run((s) => mockLeaveFamily(s, sender, args)),
    }),
    [sender, controlledNodeIds, queries, isExecuting, error, clearError, refreshAll, run],
  );

  return <FamiliesContext.Provider value={value}>{children}</FamiliesContext.Provider>;
};
