import { PaletteMode } from '@mui/material'
import {
  PaletteOptions,
  NymPalette,
  ThemeOptions,
  createTheme,
  NymPaletteVariant,
} from '@mui/material/styles'

//-----------------------------------------------------------------------------------------------
// Nym palette type definitions
//

/**
 * The Nym palette.
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const nymPalette: NymPalette = {
  /** emphasises important elements */
  highlight: '#FB6E4E',
  text: {
    nav: '#F2F2F2',
    /** footer text colour */
    footer: '#666B77',
  },
}

const darkMode: NymPaletteVariant = {
  mode: 'dark',
  background: {
    main: '#070B15',
    paper: '#242C3D',
  },
  text: {
    main: '#F2F2F2',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#242C3D',
    hover: '#111826',
  },
}

const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#F2F2F2',
    paper: '#FFFFFF',
  },
  text: {
    main: '#666666',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#242C3D',
    hover: '#111826',
  },
}

//-----------------------------------------------------------------------------------------------
// Nym palettes for light and dark mode
//

/**
 * Map a Nym palette variant onto the MUI palette
 */
const variantToMUIPalette = (variant: NymPaletteVariant): PaletteOptions => ({
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
})

/**
 * Returns the Network Explorer palette for light mode.
 */
const createLightModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
  },
  ...variantToMUIPalette(lightMode),
})

/**
 * Returns the Network Explorer palette for dark mode.
 */
const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
  },
  ...variantToMUIPalette(darkMode),
})

/**
 * IMPORANT: if you need to get the default MUI theme, use the following
 *
 *   import { createTheme as systemCreateTheme } from '@mui/system';
 *
 *   // get the MUI system defaults for light mode
 *   const systemTheme = systemCreateTheme({ palette: { mode: 'light' } });
 *
 *
 *   return {
 *     // change `primary` to default MUI `success`
 *     primary: {
 *       main: systemTheme.palette.success.main,
 *     },
 *     nym: {
 *       ...nymPalette,
 *     },
 *   };
 */

//-----------------------------------------------------------------------------------------------
// Nym theme overrides
//

/**
 * Gets the theme options to be passed to `createTheme`.
 *
 * Based on pattern from https://mui.com/customization/dark-mode/#dark-mode-with-custom-palette.
 *
 * @param mode     The theme mode: 'light' or 'dark'
 */
export const getDesignTokens = (mode: PaletteMode): ThemeOptions => {
  // first, create the palette from user's choice of light or dark mode
  const { palette } = createTheme({
    palette: {
      mode,
      ...(mode === 'light'
        ? createLightModePalette()
        : createDarkModePalette()),
    },
  })

  // then customise theme and components
  return {
    typography: {
      fontFamily: [
        'Open Sans',
        'sans-serif',
        'BlinkMacSystemFont',
        'Roboto',
        'Oxygen',
        'Ubuntu',
        'Helvetica Neue',
      ].join(','),
      fontSize: 14,
      fontWeightRegular: 600,
    },
    transitions: {
      duration: {
        shortest: 150,
        shorter: 200,
        short: 250,
        standard: 300,
        complex: 375,
        enteringScreen: 225,
        leavingScreen: 195,
      },
      easing: {
        easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
      },
    },
    palette,
  }
}
