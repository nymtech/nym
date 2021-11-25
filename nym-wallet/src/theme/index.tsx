import * as React from 'react'
import { createTheme, ThemeProvider } from '@mui/material/styles'
import { CssBaseline } from '@mui/material'
import { getDesignTokens } from './theme'

/**
 * Provides the theme for the Network Explorer by reacting to the light/dark mode choice stored in the app context.
 */
export const NymWalletTheme: React.FC = ({ children }) => {
  const theme = React.useMemo(
    () => createTheme(getDesignTokens('light')),
    ['light'],
  )
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  )
}
