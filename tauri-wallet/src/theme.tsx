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
        borderRadius: 50,
      },
      contained: {
        padding: '12px 24px',
      },
      containedSizeLarge: {
        padding: '12px 24px',
      },
    },
    MuiOutlinedInput: {
      root: {
        borderRadius: 50,
        background: '#fff',
      },
      notchedOutline: {
        margin: -2,
      },
    },
  },
})
