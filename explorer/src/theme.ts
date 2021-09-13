import { createTheme } from '@mui/material/styles';

const nymPalette = {
  primary: {
    main: '#F2F2F2', // white
    light: 'white', // tbc
    dark: '#070B15', // Black [top nav bg]
    selectedText: '#FB6E4E', // orange selected text
  },
  secondary: {
    main: '#009FA8',
    light: '#5C616D', // grey [hamburger grey]
    dark: '#242C3D', // lighter black [nav bg]
  },
  background: {
    main: '#242C3D',
    darkBlack: '#070B15',
    lightBlack: '#242C3D',
  },
};

// A custom theme for this app
export const theme = createTheme({
  typography: {
    fontFamily:
      'monospace, open sans, sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"',
    fontSize: 14,
    fontWeightBold: 600,
  },
  palette: {
    primary: {
      main: nymPalette.primary.main,
      light: nymPalette.primary.light,
      dark: nymPalette.primary.dark,
      contrastText: nymPalette.primary.selectedText,
      // only way to add an additional colour seems
      // to be `contrastText`?
    },
    secondary: {
      main: nymPalette.secondary.main,
      light: nymPalette.secondary.light,
      dark: nymPalette.secondary.dark,
    },
  },
  shape: {
    borderRadius: 24,
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
});
