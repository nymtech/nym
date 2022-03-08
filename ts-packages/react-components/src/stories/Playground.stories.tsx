import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { useTheme } from '@mui/material';
import { Playground } from '../playground';
import { PlaygroundPalette } from '../playground/theme';
import { MUIThemeExplorer } from '../playground/theme/MUIThemeExplorer';

export default {
  title: 'Playground',
  component: Playground,
} as ComponentMeta<typeof Playground>;

export function AllControls() {
  return <Playground />;
}

export function Palette() {
  return <PlaygroundPalette />;
}

export function ThemeExplorer() {
  const theme = useTheme();
  return <MUIThemeExplorer theme={theme} />;
}
