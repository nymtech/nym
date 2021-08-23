import { createTheme } from '@material-ui/core'

export const theme = createTheme({
  palette: {
    primary: {
      main: '#F4731B',
    },
  },
  overrides: {
    MuiButton: {
      root: {
        borderRadius: 50,
        padding: '12px 24px',
      },
      containedPrimary: {
        color: 'white',
      },
      text: {
        padding: 'default',
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
