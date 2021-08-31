import React from 'react'
import ReactDOM from 'react-dom'
import { CssBaseline, ThemeProvider } from '@material-ui/core'
import { BrowserRouter as Router } from 'react-router-dom'
import { Routes } from './routes'
import { theme } from './theme'
import { ClientContextProvider } from './context/main'

const App = () => (
  <ThemeProvider theme={theme}>
    <CssBaseline />
    <Router>
      <ClientContextProvider>
        <Routes />
      </ClientContextProvider>
    </Router>
  </ThemeProvider>
)

const root = document.getElementById('root')

ReactDOM.render(<App />, root)
