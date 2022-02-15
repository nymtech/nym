import React, { useContext } from 'react'
import ReactDOM from 'react-dom'
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
