/* eslint-disable react/jsx-pascal-case */
import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Alert, Box } from '@mui/material';
import { NymShipyardTheme } from 'src/theme';
import { TestAndEarnPopup, TestAndEarnPopupContent } from './TestAndEarnPopup';
import { TestAndEarnContextProvider } from './context/TestAndEarnContext';
import { MockProvider } from '../../context/mocks/main';
import { ConnectionStatusKind } from '../../types';
import { TestAndEarnCurrentDraw } from './TestAndEarnCurrentDraw';
import { TestAndEarnWinner } from './TestAndEarnWinner';
import { TestAndEarnDraws } from './TestAndEarnDraws';
import { TestAndEarnWinnerWalletAddress } from './TestAndEarnWinnerWalletAddress';
import {
  MockTestAndEarnProvider_NotRegistered,
  MockTestAndEarnProvider_Registered,
  MockTestAndEarnProvider_RegisteredAndError,
  MockTestAndEarnProvider_RegisteredWithDraws,
  MockTestAndEarnProvider_RegisteredWithDrawsAndEntry,
  MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndNoWinner,
  MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner,
  MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerClaimed,
  MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerCollectWallet,
  MockTestAndEarnProvider_RegisteredWithDrawsNoCurrent,
} from './context/mocks/TestAndEarnContext';

export default {
  title: 'Growth/TestAndEarn/Content/Popup',
  component: TestAndEarnPopupContent,
} as ComponentMeta<typeof TestAndEarnPopupContent>;

const MacOSWindow: FCWithChildren<{
  width?: string | number;
  height?: string | number;
  title?: string;
  children: React.ReactNode;
}> = ({ title, width, height, children }) => (
  <Box sx={{ border: '1px solid #EEEEEE', width, height }}>
    <Box sx={{ background: '#EEEEEE', display: 'grid', gridTemplateColumns: 'auto auto', gridTemplateRows: 'auto' }}>
      <Box ml={1}>
        <svg width="52px" height="12px" viewBox="0 0 52 12" version="1.1" xmlns="http://www.w3.org/2000/svg">
          <g id="Components" stroke="none" strokeWidth="1" fill="none" fillRule="evenodd">
            <g id="macOS" transform="translate(-600.000000, -220.000000)">
              <g id="Group" transform="translate(600.000000, 220.000000)" strokeWidth="0.5">
                <g id="Traffic-Lights">
                  <circle id="Traffic-Light---Zoom" stroke="#1BAC2C" fill="#2ACB42" cx="46" cy="6" r="5.75" />
                  <circle id="Traffic-Light---Minimise" stroke="#DFA023" fill="#FFC12F" cx="26" cy="6" r="5.75" />
                  <circle id="Traffic-Light---Close" stroke="#E24640" fill="#FF6157" cx="6" cy="6" r="5.75" />
                </g>
              </g>
            </g>
          </g>
        </svg>
      </Box>
      <Box
        sx={{
          alignSelf: 'center',
          color: '#000000',
          opacity: 0.848675272,
          fontSize: 13,
        }}
      >
        {title || 'Window title'}
      </Box>
    </Box>
    <Box sx={{ overflowY: 'scroll', height: 'calc(100% - 25px)' }}>{children}</Box>
  </Box>
);

const Wrapper: FCWithChildren<{ text: React.ReactNode }> = ({ text }) => (
  <NymShipyardTheme>
    <Alert severity="info" sx={{ mb: 4 }}>
      {text}
    </Alert>
    <MacOSWindow width={700} height={600} title="Test&Earn">
      <TestAndEarnPopup />
    </MacOSWindow>
  </NymShipyardTheme>
);

export const Stage0 = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_NotRegistered>
      <Wrapper text="The user sees this content when they have not joined Test&Earn." />
    </MockTestAndEarnProvider_NotRegistered>
  </MockProvider>
);

export const Stage1EnterDraw = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDraws>
      <Wrapper text="The user has signed up and can see the next draw and choose the enter." />
    </MockTestAndEarnProvider_RegisteredWithDraws>
  </MockProvider>
);

export const Stage2GetTask = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDrawsAndEntry>
      <Wrapper text="The user has entered a draw and can view the word of the day if they missed the popup notification." />
    </MockTestAndEarnProvider_RegisteredWithDrawsAndEntry>
  </MockProvider>
);

export const Stage3Winner = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner>
      <Wrapper text="The user has won and can claim their prize." />
    </MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner>
  </MockProvider>
);

export const Stage3NoPrize = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndNoWinner>
      <Wrapper text="The user has not won. A winner has been announced." />
    </MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndNoWinner>
  </MockProvider>
);

export const Stage4EnterWalletAddress = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerCollectWallet>
      <Wrapper text="The user is a winner, claims their prize and enters their wallet address." />
    </MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerCollectWallet>
  </MockProvider>
);

export const Stage5ClaimedPrize = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerClaimed>
      <Wrapper text="The user is a winner and has claimed their prize." />
    </MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinnerClaimed>
  </MockProvider>
);

export const Stage6DrawsFinished = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connected}>
    <MockTestAndEarnProvider_RegisteredWithDrawsNoCurrent>
      <Wrapper text="There are no more draws. The user can see their entries and prizes they have claimed." />
    </MockTestAndEarnProvider_RegisteredWithDrawsNoCurrent>
  </MockProvider>
);

export const Connecting = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.connecting}>
    <TestAndEarnContextProvider>
      <Wrapper text="Test&Earn requires the user to be connected to talk the API. This is shown while connecting." />
    </TestAndEarnContextProvider>
  </MockProvider>
);

export const Disconnected = () => (
  <MockProvider connectionStatus={ConnectionStatusKind.disconnected}>
    <TestAndEarnContextProvider>
      <Wrapper text="Test&Earn requires the user to be connected to talk the API. This is shown when not connected." />
    </TestAndEarnContextProvider>
  </MockProvider>
);

export const Error = () => (
  <MockProvider>
    <MockTestAndEarnProvider_RegisteredAndError>
      <Wrapper text="The user see this with details about errors. They can submit an error report." />
    </MockTestAndEarnProvider_RegisteredAndError>
  </MockProvider>
);
