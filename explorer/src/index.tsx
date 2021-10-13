import * as React from 'react';
import ReactDOM from 'react-dom';
import { BrowserRouter as Router } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import { CssBaseline } from '@mui/material';
import { App } from './App';
import { MainContext, MainContextProvider } from './context/main';
import './styles.css';

const palette = {
  primary: {
    main: '#F2F2F2', // white text in nav etc
    dark: '#070B15', // background black in nav appbar
    light: '#FFFFFF' // white bg cards
  },
  secondary: {
    main: '#666666', // grey text
    dark: '#242C3D', // drawer slide out
    light: '#F2F2F2' // grey bg
  },
  brandOrange: '#FB6E4E',
}
const AppWrapper = () => {
  const { mode }: any = React.useContext(MainContext);

  const theme = createTheme({
    palette: {
      mode,
      ...palette,
      ...(mode === 'light' ? {
        background: {
          default: palette.secondary.light
        }} : {
          background: {
            default: '#111826',
          }
        }
      ),
    },
    typography: {
      fontFamily:
        'open sans, sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"',
      fontSize: 14,
      fontWeightBold: 600,
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
      MuiCardHeader: {
        styleOverrides: {
          title: {
            fontSize: '16px',
            fontWeight: 'bold'
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            background: palette.secondary.dark,
          }
        }
      },
      MuiListItem: {
        styleOverrides: {
          root: {
            background: palette.secondary.dark,
            // '&:hover': {
            //   backgroundColor: 'transparent',
            // }
          }
        }
      }
    },
  });

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Router>
        <App />
      </Router>
    </ThemeProvider>
  );
};

ReactDOM.render(
  <MainContextProvider>
    <AppWrapper />
  </MainContextProvider>,
  document.getElementById('app'),
);
