import { createTheme } from '@material-ui/core'

export const theme = createTheme({
  palette: {
    primary: {
      main: '#F4731B',
    },
  },
  overrides: {
    MuiButton: {
      containedPrimary: {
        color: 'white',
      },
    },
  },
})
