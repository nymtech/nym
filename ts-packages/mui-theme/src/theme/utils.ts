import { PaletteOptions } from '@mui/material/styles';
import { darkMode, lightMode, nymPalette, variantToMUIPalette } from './common';
import { nymWalletPallete } from './wallet';
// eslint-disable-next-line import/no-cycle
import { networkExplorerPalette } from './network-explorer';
// eslint-disable-next-line import/no-cycle
import { NymPaletteWithExtensionsOptions } from './theme';

/**
 * Returns the Nym palette for light mode.
 */
export const createLightModePalette = (): PaletteOptions & NymPaletteWithExtensionsOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPallete(lightMode),
    ...networkExplorerPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

/**
 * Returns the Nym palette for dark mode.
 */
export const createDarkModePalette = (): PaletteOptions & NymPaletteWithExtensionsOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPallete(darkMode),
    ...networkExplorerPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
});
