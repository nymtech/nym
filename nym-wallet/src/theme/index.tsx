import React, { useContext } from 'react'
import { createTheme, ThemeProvider } from '@mui/material/styles'
import { CssBaseline } from '@mui/material'
import { getDesignTokens } from './theme'
import { ClientContext } from '../context/main'

/**
 * Provides the theme for the Network Explorer by reacting to the light/dark mode choice stored in the app context.
 */
export const NymWalletTheme: React.FC = ({ children }) => {
  const { mode } = useContext(ClientContext)
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode])
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  )
}

export const WelcomeTheme: React.FC = ({ children }) => {
  const theme = createTheme(getDesignTokens('dark'))
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  )
}
