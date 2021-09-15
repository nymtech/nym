import { createTheme } from '@mui/material/styles';

const nymPalette = {
  // DARK THEME
  primary: {
    main: '#F2F2F2', // white, font, text, main text color
    light: '#242C3D', // side nav, cards etc.
    dark: '#111826', // main window background
    darker: '#070B15', // top nav/app bar
    selectedText: '#fd9d35',
  },
  // LIGHT THEME
  secondary: {
    main: '#000', // white, font, text, main text color
    light: '#fff', // side nav, cards etc.
    dark: '#808080', // main window background
    darker: '#a9a9a9', // top nav/app bar
    selectedText: '#fd9d35',
  },
  background: {
    main: '#242C3D',
    darkBlack: '#070B15',
    lightBlack: '#242C3D',
    selectedText: '#FB6E4E', // orange selected text
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
    MuiLink: {
      styleOverrides: {
        root: {
          textDecoration: 'none',
        },
      },
    },
    MuiSvgIcon: {
      styleOverrides: {
        root: {
          color: nymPalette.primary.main,
        },
      },
    },
    MuiDrawer: {
      styleOverrides: {
        root: {
          color: nymPalette.primary.light,
          backgroundColor: nymPalette.primary.light,
        },
        paper: {
          color: nymPalette.primary.light,
          backgroundColor: nymPalette.primary.light,
        },
      },
    },
    MuiToolbar: {
      styleOverrides: {
        root: {
          backgroundColor: nymPalette.primary.darker,
          color: nymPalette.primary.main,
        },
      },
    },
  },
});
