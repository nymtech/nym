import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { useTheme } from '@mui/material';
import { Playground } from '../playground/Playground';
import { PlaygroundPalette } from '../playground/theme';
import { MUIThemeExplorer } from '../playground/theme/MUIThemeExplorer';

export default {
  title: 'Playground',
  component: Playground,
} as ComponentMeta<typeof Playground>;

export const AllControls = () => <Playground />;

export const Palette = () => <PlaygroundPalette />;

export const ThemeExplorer = () => {
  const theme = useTheme();
  return <MUIThemeExplorer theme={theme} />;
};
