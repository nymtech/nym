import { PaletteMode } from '@mui/material';
import { PaletteOptions } from '@mui/material/styles';

/**
 * This interface defines a palette used across Nym for branding
 */
export interface NymPalette {
  highlight: string;

  status: {
    success: string;
    info: string;
  };

  light: string;
  dark: string;

  muted: {
    onDarkBg: string;
  };
}

/**
 * This interface defines the palette for a light or dark mode variant
 */
export interface NymPaletteVariant {
  mode: PaletteMode;
  background: {
    main: string;
    paper: string;
  };
  text: {
    main: string;
  };
  topNav: {
    background: string;
  };
  nav: {
    text: string;
    background: string;
    hover: string;
  };
  mixnodes: {
    status: {
      active: string;
      standby: string;
    };
  };
  selectionChance: {
    overModerate: string;
    moderate: string;
    underModerate: string;
  };
}

// -------------------------------------------------------------------------------------------------------------------

/**
 * The Nym palette.
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
export const nymPalette: NymPalette = {
  /** emphasises important elements */
  highlight: '#FB6E4E',

  /** statuses */
  status: {
    success: '#21D073',
    info: '#60D7EF',
  },

  /** light and dark base values */
  light: '#F4F6F8',
  dark: '#121726',

  /** muted on backgrounds */
  muted: {
    onDarkBg: '#666B77',
  },
};

// -------------------------------------------------------------------------------------------------------------------

/**
 * Dark mode variant
 */
export const darkMode: NymPaletteVariant = {
  mode: 'dark',
  background: {
    main: nymPalette.dark,
    paper: '#242C3D',
  },
  text: {
    main: '#F2F2F2',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    text: '#F2F2F2',
    background: '#242C3D',
    hover: '#111826',
  },
  mixnodes: {
    status: {
      active: '#20D073',
      standby: '#5FD7EF',
    },
  },
  selectionChance: {
    overModerate: '#20D073',
    moderate: '#EBA53D',
    underModerate: '#DA465B',
  },
};

/**
 * Light mode variant
 */
export const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#F2F2F2',
    paper: '#FFFFFF',
  },
  text: {
    main: '#121726',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    text: '#F2F2F2',
    background: '#242C3D',
    hover: '#111826',
  },
  mixnodes: {
    status: {
      active: '#1CBB67',
      standby: '#55C1D7',
    },
  },
  selectionChance: {
    overModerate: '#20D073',
    moderate: '#EBA53D',
    underModerate: '#DA465B',
  },
};

/**
 * Map a Nym palette variant onto the MUI palette
 */
export const variantToMUIPalette = (variant: NymPaletteVariant): PaletteOptions => ({
  text: {
    primary: variant.text.main,
  },
  primary: {
    main: nymPalette.highlight,
    contrastText: '#fff',
  },
  background: {
    default: variant.background.main,
    paper: variant.background.paper,
  },
});
