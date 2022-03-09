import * as React from 'react';
import { useTheme } from '@mui/material';
import { MUIThemeExplorer } from './theme/MUIThemeExplorer';
import { PaletteSwatches, PaletteSwatchesList } from './theme/PaletteSwatches';

export const PlaygroundTheme: React.FC = () => {
  const theme = useTheme();
  return (
    <>
      <h3>Palette</h3>
      <PaletteSwatches theme={theme} />
      <h3>Palette Explorer</h3>
      <MUIThemeExplorer theme={theme} />
    </>
  );
};

export const PlaygroundPalette: React.FC = () => {
  const theme = useTheme();
  return <PaletteSwatchesList theme={theme} />;
};
