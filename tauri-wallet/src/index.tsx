import React, { useContext } from 'react'
import ReactDOM from 'react-dom'
import { ErrorBoundary } from 'react-error-boundary'
import { CssBaseline, ThemeProvider } from '@material-ui/core'
import { BrowserRouter as Router } from 'react-router-dom'
import { Routes } from './routes'
import { theme } from './theme'
import { ClientContext, ClientContextProvider } from './context/main'
import { ApplicationLayout } from './layouts'
import { SignIn } from './routes/sign-in'
import { Admin, ErrorFallback } from './components'

const AppWrapper = () => {
  const { clientDetails } = useContext(ClientContext)
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {!clientDetails ? (
        <SignIn />
      ) : (
        <ApplicationLayout>
          <>
            <Admin />
            <Routes />
          </>
        </ApplicationLayout>
      )}
    </ThemeProvider>
  )
}

const App = () => {
  return (
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <Router>
        <ClientContextProvider>
          <AppWrapper />
        </ClientContextProvider>
      </Router>
    </ErrorBoundary>
  )
}

const root = document.getElementById('root')

ReactDOM.render(<App />, root)
