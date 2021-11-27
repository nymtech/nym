import * as React from 'react'
import { createTheme, ThemeProvider } from '@mui/material/styles'
import { CssBaseline } from '@mui/material'
import { getDesignTokens } from './theme'

export const NymThemeProvider: React.FC = ({ children }) => {
  const theme = createTheme(getDesignTokens('dark'))
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  )
}
