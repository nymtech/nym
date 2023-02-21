/* eslint-disable @typescript-eslint/naming-convention */
import React from 'react';
import { DateTime } from 'luxon';
import { TTestAndEarnContext, TestAndEarnContext } from '../TestAndEarnContext';
import { DrawEntry, DrawEntryStatus, DrawWithWordOfTheDay } from '../types';

const methodDefaults = {
  loadedOnce: true,
  loading: false,
  refresh: async () => undefined,
  setAndStoreClientId: async () => undefined,
  setAndStoreRegistration: async () => undefined,
  clearStorage: async () => undefined,
  toggleGrowthWindow: async () => undefined,
  setWalletAddress: async () => undefined,
  enterDraw: async () => ({} as DrawEntry),
  claim: async () => undefined,
};

const mockValues_NotRegistered: TTestAndEarnContext = {
  ...methodDefaults,
};

export const MockTestAndEarnProvider_NotRegistered = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_NotRegistered}>{children}</TestAndEarnContext.Provider>
);

export const testMarkdown = `**Create a sentence including "Nym" and one or more of the following words** *(in any language)*:
 
- Privacy
- Pleasure 
- Pineapple
- Mix
`;

const mockValues_Registered: TTestAndEarnContext = {
  ...methodDefaults,
  registration: {
    id: '1234',
    client_id_signature: 'signature',
    client_id: '5678',
    timestamp: '2022-12-12T18:17:37.840Z',
  },
};

export const MockTestAndEarnProvider_Registered = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_Registered}>{children}</TestAndEarnContext.Provider>
);

const allDraws: DrawEntry[] = [
  {
    draw_id: '1111',
    timestamp: DateTime.now().toISO(),
    id: 'AAAA',
    status: DrawEntryStatus.pending,
  },
  {
    draw_id: '2222',
    timestamp: DateTime.now().toISO(),
    id: 'BBBB',
    status: DrawEntryStatus.noWin,
  },
  {
    draw_id: '2222',
    timestamp: DateTime.now().toISO(),
    id: 'BBBB',
    status: DrawEntryStatus.claimed,
  },
  {
    draw_id: '2222',
    timestamp: DateTime.now().toISO(),
    id: 'BBBB',
    status: DrawEntryStatus.winner,
  },
];

const draws: DrawEntry[] = [
  {
    draw_id: '1111',
    timestamp: DateTime.now().toISO(),
    id: 'AAAA',
    status: DrawEntryStatus.pending,
  },
  {
    draw_id: '2222',
    timestamp: DateTime.now().toISO(),
    id: 'BBBB',
    status: DrawEntryStatus.noWin,
  },
];

const drawsWithWin: DrawEntry[] = [
  {
    draw_id: '1111',
    timestamp: DateTime.now().toISO(),
    id: 'AAAA',
    status: DrawEntryStatus.winner,
  },
  {
    draw_id: '2222',
    timestamp: DateTime.now().toISO(),
    id: 'BBBB',
    status: DrawEntryStatus.noWin,
  },
];

const drawsWithClaim: DrawEntry[] = [
  {
    draw_id: '1111',
    timestamp: DateTime.now().toISO(),
    id: 'AAAA',
    status: DrawEntryStatus.claimed,
  },
  {
    draw_id: '2222',
    timestamp: DateTime.now().toISO(),
    id: 'BBBB',
    status: DrawEntryStatus.noWin,
  },
];

const current: DrawWithWordOfTheDay = {
  id: '1111',
  start_utc: DateTime.now().toISO(),
  end_utc: DateTime.now().plus({ day: 1 }).minus({ second: 25 }).toISO(),
  last_modified: DateTime.now().toISO(),
  word_of_the_day: testMarkdown,
};

const mockValues_RegisteredWithAllDrawsAndEntry: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    current: {
      ...current,
    },
    draws: allDraws,
  },
};

export const MockTestAndEarnProvider_RegisteredWithAllDraws = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithAllDrawsAndEntry}>
    {children}
  </TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDrawsNoCurrent: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    draws: drawsWithClaim,
  },
};

export const MockTestAndEarnProvider_RegisteredWithDrawsNoCurrent = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDrawsNoCurrent}>{children}</TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDraws: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    current,
    draws,
  },
};

export const MockTestAndEarnProvider_RegisteredWithDraws = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDraws}>{children}</TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDrawsAndEntry: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    current: {
      ...current,
      entry: mockValues_RegisteredWithDraws.draws!.draws[0],
    },
    draws,
  },
};

export const MockTestAndEarnProvider_RegisteredWithDrawsAndEntry = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDrawsAndEntry}>{children}</TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDrawsAndEntryAndWinner: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    current: {
      ...current,
      entry: drawsWithWin[0],
    },
    draws: drawsWithWin,
  },
  isWinnerWithUnclaimedPrize: true,
};

export const MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner = ({
  children,
}: {
  children: React.ReactNode;
}) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDrawsAndEntryAndWinner}>
    {children}
  </TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDrawsAndEntryAndNoWinner: TTestAndEarnContext = {
  ...mockValues_RegisteredWithDrawsAndEntry,
  isWinnerWithUnclaimedPrize: false,
};

export const MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndNoWinner = ({
  children,
}: {
  children: React.ReactNode;
}) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDrawsAndEntryAndNoWinner}>
    {children}
  </TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDrawsAndEntryAndWinnerCollectWallet: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    draws: drawsWithWin,
  },
};

export const MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerCollectWallet = ({
  children,
}: {
  children: React.ReactNode;
}) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDrawsAndEntryAndWinnerCollectWallet}>
    {children}
  </TestAndEarnContext.Provider>
);

const mockValues_RegisteredWithDrawsAndEntryAndWinnerClaimed: TTestAndEarnContext = {
  ...mockValues_Registered,
  draws: {
    draws: drawsWithClaim,
  },
};

export const MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerClaimed = ({
  children,
}: {
  children: React.ReactNode;
}) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredWithDrawsAndEntryAndWinnerClaimed}>
    {children}
  </TestAndEarnContext.Provider>
);

const mockValues_RegisteredAndError: TTestAndEarnContext = {
  ...mockValues_Registered,
  error: 'Error message text will go here',
};

export const MockTestAndEarnProvider_RegisteredAndError = ({ children }: { children: React.ReactNode }) => (
  <TestAndEarnContext.Provider value={mockValues_RegisteredAndError}>{children}</TestAndEarnContext.Provider>
);
