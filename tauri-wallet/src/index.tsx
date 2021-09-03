import React, { useContext } from 'react'
import ReactDOM from 'react-dom'
import { CssBaseline, ThemeProvider } from '@material-ui/core'
import { BrowserRouter as Router } from 'react-router-dom'
import { Routes } from './routes'
import { theme } from './theme'
import { ClientContext, ClientContextProvider } from './context/main'
import { ApplicationLayout } from './layouts'
import { SignIn } from './routes/sign-in'

const AppWrapper = () => {
  const { clientDetails } = useContext(ClientContext)
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {!clientDetails ? (
        <SignIn />
      ) : (
        <ApplicationLayout>
          <Routes />
        </ApplicationLayout>
      )}
    </ThemeProvider>
  )
}

const App = () => {
  return (
    <Router>
      <ClientContextProvider>
        <AppWrapper />
      </ClientContextProvider>
    </Router>
  )
}

const root = document.getElementById('root')

ReactDOM.render(<App />, root)
