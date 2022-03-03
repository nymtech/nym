import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymThemeProvider } from '@nymproject/mui-theme';
import { Playground } from '../playground';

export default {
  title: 'Playground',
  component: Playground,
} as ComponentMeta<typeof Playground>;

export function LightMode() {
  return (
    <NymThemeProvider mode="light">
      <Playground />
    </NymThemeProvider>
  );
}

export function DarkMode() {
  return (
    <NymThemeProvider mode="dark">
      <Playground />
    </NymThemeProvider>
  );
}
