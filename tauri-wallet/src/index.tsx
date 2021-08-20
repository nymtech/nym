import React from 'react'
import ReactDOM from 'react-dom'
import { CssBaseline, ThemeProvider } from '@material-ui/core'
import { Routes } from './routes'
import { theme } from './theme'
import { ClientContextProvider } from './context/main'

const App = () => (
  <ThemeProvider theme={theme}>
    <CssBaseline />
    <ClientContextProvider>
      <Routes />
    </ClientContextProvider>
  </ThemeProvider>
)

const root = document.getElementById('root')

ReactDOM.render(<App />, root)
