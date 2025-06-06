/* eslint-disable no-shadow,@typescript-eslint/no-unused-vars,@typescript-eslint/no-empty-interface */
import { Theme, ThemeOptions, Palette, PaletteOptions } from '@mui/material/styles';
import { PaletteMode } from '@mui/material';

/**
 * If you are unfamiliar with Material UI theming, please read the following first:
 * - https://mui.com/customization/theming/
 * - https://mui.com/customization/palette/
 * - https://mui.com/customization/dark-mode/#dark-mode-with-custom-palette
 *
 * This file adds typings to the theme using Typescript's module augmentation.
 *
 * Read the following if you are unfamiliar with module augmentation and declaration merging. Then
 * look at the recommendations from Material UI docs for implementation:
 * - https://www.typescriptlang.org/docs/handbook/declaration-merging.html#module-augmentation
 * - https://www.typescriptlang.org/docs/handbook/declaration-merging.html#merging-interfaces
 * - https://mui.com/customization/palette/#adding-new-colors
 *
 *
 * IMPORTANT:
 *
 * The type augmentation must match MUI's definitions. So, notice the use of `interface` rather than
 * `type Foo = { ... }` - this is necessary to merge the definitions.
 */

declare module '@mui/material/styles' {
  /**
   * This interface defines a palette used across Nym for branding
   */
  interface NymPalette {
    highlight: string;
    success: string;
    info: string;
    red: string;
    fee: string;
    background: { light: string; dark: string };
    text: {
      light: string;
      dark: string;
      muted: string;
      grey: string;
    };
    linkHover: string;
    border: { menu: string };
  }

  interface NymPaletteVariant {
    mode: PaletteMode;
    background: {
      main: string;
      paper: string;
      warn: string;
      grey: string;
      greyStroke: string;
      elevated: string; // New property for more depth options
      subtle: string; // New property for subtle backgrounds
    };
    text: {
      main: string;
      muted: string;
      warn: string;
      contrast: string;
      grey: string;
      blue: string;
      subdued: string; // New property for better text hierarchy
    };
    topNav: {
      background: string;
    };
    nav: {
      background: string;
    };
    hover: {
      background: string;
    };
    modal: {
      border: string;
    };
    chart: {
      grey: string;
    };
    // New modern properties
    gradients: {
      primary: string;
      subtle: string;
    };
    shadows: {
      light: string;
      medium: string;
      strong: string;
    };
  }

  /**
   * A palette definition only for the Nym Wallet that extends the Nym palette
   */
  interface NymWalletPalette {
    nymWallet: NymPaletteVariant;
  }

  interface NymPaletteAndNymWalletPalette {
    nym: NymPalette & NymWalletPalette;
  }

  type NymPaletteAndNymWalletPaletteOptions = Partial<NymPaletteAndNymWalletPalette>;

  /**
   * Add anything not palette related to the theme here
   */
  interface NymTheme {}

  /**
   * This augments the definitions of the MUI Theme with the Nym theme, as well as
   * a partial `ThemeOptions` type used by `createTheme`
   *
   * IMPORTANT: only add extensions to the interfaces above, do not modify the lines below
   */
  interface Theme extends NymTheme {}
  interface ThemeOptions extends Partial<NymTheme> {}
  interface Palette extends NymPaletteAndNymWalletPalette {}
  interface PaletteOptions extends NymPaletteAndNymWalletPaletteOptions {}
}
