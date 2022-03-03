import React, { useContext, useEffect, useLayoutEffect } from 'react'
import ReactDOM from 'react-dom'
import { ErrorBoundary } from 'react-error-boundary'
import { BrowserRouter as Router } from 'react-router-dom'
import { SnackbarProvider } from 'notistack'
import { Routes } from './routes'
import { ClientContext, ClientContextProvider } from './context/main'
import { ApplicationLayout } from './layouts'
import { Admin, Welcome } from './pages'
import { ErrorFallback } from './components'
import { NymWalletTheme, WelcomeTheme } from './theme'
import { Settings } from './pages'
import { maximizeWindow } from './utils'

const App = () => {
  const { clientDetails } = useContext(ClientContext)

  useLayoutEffect(() => {
    maximizeWindow()
  }, [])

  return !clientDetails ? (
    <WelcomeTheme>
      <Welcome />
    </WelcomeTheme>
  ) : (
    <NymWalletTheme>
      <ApplicationLayout>
        <Settings />
        <Admin />
        <Routes />
      </ApplicationLayout>
    </NymWalletTheme>
  )
}

const AppWrapper = () => {
  return (
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <Router>
        <SnackbarProvider
          anchorOrigin={{
            vertical: 'bottom',
            horizontal: 'right',
          }}
        >
          <ClientContextProvider>
            <App />
          </ClientContextProvider>
        </SnackbarProvider>
      </Router>
    </ErrorBoundary>
  )
}

const root = document.getElementById('root')

ReactDOM.render(<AppWrapper />, root)
