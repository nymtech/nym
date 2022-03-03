import { darkMode, nymPalette, NymPaletteVariant } from './common';

/**
 * A palette definition only for the Network Explorer that extends the Nym palette
 */

export interface NetworkExplorerPalette {
  networkExplorer: {
    map: {
      stroke: string;
      fills: string[];
    };
    background: {
      tertiary: string;
    };
    topNav: {
      background: string;
      socialIcons: string;
      appBar: string;
    };
    nav: {
      selected: {
        main: string;
        nested: string;
      };
      background: string;
      hover: string;
      text: string;
    };
    footer: {
      socialIcons: string;
    };
    mixnodes: {
      status: {
        active: string;
        standby: string;
        inactive: string;
      };
    };
  };
}

/**
 * Nym palette specific to the Network Explorer
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
export const networkExplorerPalette = (variant: NymPaletteVariant): NetworkExplorerPalette => ({
  networkExplorer: {
    /** world map styles */
    map: {
      stroke: '#333333',
      fills: ['rgba(255,255,255,0.2)', '#EFEFEF', '#FBE7E1', '#F7D1C6', '#F09379'],
    },
    background: {
      tertiary: variant.mode === 'light' ? '#F4F8FA' : '#323C51',
    },
    /** left nav styles */
    nav: {
      selected: {
        main: '#111826',
        nested: '#3C4558',
      },
      background: variant.nav.background,
      hover: variant.nav.hover,
      text: variant.nav.text,
    },
    topNav: {
      ...variant.topNav,
      appBar: '#080715',
      socialIcons: '#F2F2F2',
    },
    footer: {
      socialIcons: variant.mode === 'light' ? nymPalette.muted.onDarkBg : darkMode.text.main,
    },
    mixnodes: {
      status: {
        active: variant.mixnodes.status.active,
        standby: variant.mixnodes.status.standby,
        inactive: variant.text.main,
      },
    },
  },
});
