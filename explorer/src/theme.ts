import { createTheme } from '@material-ui/core/styles';

const nymPalette = {
  primary: {
    main: '#F4731B',
  },
  secondary: {
    main: '#009FA8',
  },
  background: {
    main: '#121726',
  },
};

// A custom theme for this app
export const theme = createTheme({
  typography: {
    fontFamily:
      '-apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"',
    fontSize: 16,
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
