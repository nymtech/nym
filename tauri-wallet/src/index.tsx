import React, { useContext } from 'react'
import ReactDOM from 'react-dom'
import { BrowserRouter as Router } from 'react-router-dom'
import { ErrorBoundary } from 'react-error-boundary'
import { CssBaseline, ThemeProvider } from '@material-ui/core'
import { Routes } from './routes'
import { theme } from './theme'
import { ClientContext, ClientContextProvider } from './context/main'
import { ApplicationLayout } from './layouts'
import { SignIn } from './routes/sign-in'
import { Admin, ErrorFallback } from './components'
import { SnackbarProvider } from 'notistack'

const Pages = () => {
  const { clientDetails } = useContext(ClientContext)
  return (
    <>
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
    </>
  )
}

const App = () => {
  return (
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <ThemeProvider theme={theme}>
        <SnackbarProvider maxSnack={3}>
          <CssBaseline />
          <Router>
            <ClientContextProvider>
              <Pages />
            </ClientContextProvider>
          </Router>
        </SnackbarProvider>
      </ThemeProvider>
    </ErrorBoundary>
  )
}

const root = document.getElementById('root')

ReactDOM.render(<App />, root)
