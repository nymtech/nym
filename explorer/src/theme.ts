import { createTheme } from '@mui/material/styles';

const nymPalette = {
  primary: {
    main: '#F4731B',
  },
  secondary: {
    main: '#009FA8',
  },
  background: {
    main: '#242C3D',
    darkBlack: '#070B15',
    lightBlack: '#242C3D',
    // main: '#121726',
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
    },
    secondary: {
      main: nymPalette.secondary.main,
    },
  },
  shape: {
    borderRadius: 24,
  },
});
