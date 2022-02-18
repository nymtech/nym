import React, { useContext, useEffect, useLayoutEffect } from 'react'
import ReactDOM from 'react-dom'
import { appWindow, LogicalSize } from '@tauri-apps/api/window'
import { ErrorBoundary } from 'react-error-boundary'
import { BrowserRouter as Router } from 'react-router-dom'
import { Routes } from './routes'
import { ClientContext, ClientContextProvider } from './context/main'
import { ApplicationLayout } from './layouts'
import { Admin, Welcome } from './pages'
import { ErrorFallback } from './components'
import { NymWalletTheme, WelcomeTheme } from './theme'
import { Settings } from './pages'

const App = () => {
  const { clientDetails } = useContext(ClientContext)
  const setWindowSize = async () => {
    await appWindow.setSize(new LogicalSize(screen.width, screen.height))
  }

  useLayoutEffect(() => {
    setWindowSize()
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
        <ClientContextProvider>
          <App />
        </ClientContextProvider>
      </Router>
    </ErrorBoundary>
  )
}

const root = document.getElementById('root')

ReactDOM.render(<AppWrapper />, root)
