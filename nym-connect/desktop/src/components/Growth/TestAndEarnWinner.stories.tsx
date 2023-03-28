/* eslint-disable react/jsx-pascal-case */
import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymShipyardTheme } from 'src/theme';
import { TestAndEarnWinner, TestAndEarnWinnerWithState } from './TestAndEarnWinner';
import { MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner } from './context/mocks/TestAndEarnContext';

export default {
  title: 'Growth/TestAndEarn/Components/Cards/Winner',
  component: TestAndEarnWinner,
} as ComponentMeta<typeof TestAndEarnWinner>;

export const Winner = () => (
  <NymShipyardTheme>
    <MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner>
      <TestAndEarnWinnerWithState />
    </MockTestAndEarnProvider_RegisteredWithDrawsAndEntryAndWinner>
  </NymShipyardTheme>
);
