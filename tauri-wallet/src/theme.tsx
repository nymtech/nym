import { createTheme } from '@material-ui/core'

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
}

export const theme = createTheme({
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
  overrides: {
    MuiButton: {
      root: {
        padding: '12px 24px',
      },
      containedPrimary: {
        color: 'white',
      },
      text: {
        padding: '12px 24px',
      },
    },

    MuiStepIcon: {
      text: {
        fill: '#fff',
      },
    },

    MuiTooltip: {
      tooltipPlacementBottom: {
        background: nymPalette.background.main,
        padding: '8px 12px',
        fontSize: 12,
      },
    },
  },
})
