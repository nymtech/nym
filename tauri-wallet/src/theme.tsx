import { createTheme } from '@material-ui/core'

export const theme = createTheme({
  shape: {
    borderRadius: 24,
  },
  palette: {
    primary: {
      main: '#F4731B',
    },
    secondary: {
      main: '#009FA8',
    },
  },
  overrides: {
    MuiButton: {
      root: {
        padding: '12px 24px',
      },
      containedPrimary: {
        color: 'white',
      },
      text: {
        padding: 'default',
      },
    },

    MuiStepIcon: {
      text: {
        fill: '#fff',
      },
    },
  },
})
