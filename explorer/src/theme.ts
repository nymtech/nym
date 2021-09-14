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
      'open sans, sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"',
    fontSize: 14,
    fontWeightBold: 600,
  },
  palette: {
    primary: {
      main: nymPalette.primary.main,
      light: nymPalette.primary.light,
      dark: nymPalette.primary.dark,
      contrastText: nymPalette.primary.selectedText,
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
  components: {
    MuiIconButton: {
      styleOverrides: {
        root: {
          color: nymPalette.primary.light,
        },
      },
    },
    MuiSvgIcon: {
      styleOverrides: {
        root: {
          color: nymPalette.primary.light,
        },
      },
    },
    MuiDrawer: {
      styleOverrides: {
        root: {
          color: nymPalette.primary.light,
          backgroundColor: nymPalette.secondary.dark,
        },
        paper: {
          color: nymPalette.primary.light,
          backgroundColor: nymPalette.secondary.dark,
        },
      },
    },
    MuiToolbar: {
      styleOverrides: {
        root: {
          backgroundColor: nymPalette.primary.dark,
          color: nymPalette.primary.light,
        },
      },
    },
  },
});
