import * as React from 'react';
import ReactDOM from 'react-dom';
import { BrowserRouter as Router } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import { CssBaseline } from '@mui/material';
import { App } from './App';
import { DarkModeContext, DarkModeProvider } from './context/dark-mode';
import './styles.css';
import { ApiDataProvider } from './context/api';

const AppWrapper = () => {
  const { mode }: any = React.useContext(DarkModeContext);

  const theme = createTheme({
    palette: {
      mode,
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
          },
        },
      },
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
  <DarkModeProvider>
    <ApiDataProvider>
      <AppWrapper />
    </ApiDataProvider>
  </DarkModeProvider>,
  document.getElementById('app'),
);
