/* eslint-disable react/jsx-pascal-case */
import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymShipyardTheme } from 'src/theme';
import { TestAndEarnDraws } from './TestAndEarnDraws';
import { MockTestAndEarnProvider_RegisteredWithAllDraws } from './context/mocks/TestAndEarnContext';

export default {
  title: 'Growth/TestAndEarn/Components/Cards/Draws',
  component: TestAndEarnDraws,
} as ComponentMeta<typeof TestAndEarnDraws>;

export const Draws = () => (
  <NymShipyardTheme>
    <MockTestAndEarnProvider_RegisteredWithAllDraws>
      <TestAndEarnDraws />
    </MockTestAndEarnProvider_RegisteredWithAllDraws>
  </NymShipyardTheme>
);
