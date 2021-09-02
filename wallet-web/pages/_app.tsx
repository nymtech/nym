import React, { useState } from 'react'
import { Alert, AlertTitle } from '@material-ui/lab'
import Head from 'next/head'
import { ThemeProvider } from '@material-ui/core/styles'
import CssBaseline from '@material-ui/core/CssBaseline'
import { theme } from '../lib/theme'
import type { AppProps } from 'next/app'
import { ValidatorClientContext } from '../contexts/ValidatorClient'
import { Close } from '@material-ui/icons'
import { IconButton } from '@material-ui/core'

// TODO: should it perhaps be pulled from some config or also user provided?
export const BONDING_CONTRACT_ADDRESS: string =
  'punk10pyejy66429refv3g35g2t7am0was7yalwrzen'
export const VALIDATOR_URLS: string[] = [
  'https://testnet-milhon-validator1.nymtech.net',
  'https://testnet-milhon-validator2.nymtech.net',
]
export const ADDRESS_LENGTH: number = 43
export const ADMIN_ADDRESS: string =
  'punk1h3w4nj7kny5dfyjw2le4vm74z03v9vd4dstpu0'
export const DENOM: string = 'punk' // used everywhere else
export const KEY_LENGTH: number = 32
export const UDENOM: string = 'upunk' // required for client and coin construction

export default function Application(props: AppProps) {
  const { Component, pageProps } = props

  const [client, setClient] = useState(null)
  const [showAlert, setShowAlert] = useState(true)

  React.useEffect(() => {
    const jssStyles = document.querySelector('#jss-server-side')
    if (jssStyles) {
      jssStyles.parentElement.removeChild(jssStyles)
    }
  }, [])

  return (
    <React.Fragment>
      <Head>
        <meta charSet="utf-8" />
        <meta
          name="viewport"
          content="minimum-scale=1, initial-scale=1, width=device-width"
        />
        <title>Nym</title>
      </Head>
      <ValidatorClientContext.Provider value={{ client, setClient }}>
        <ThemeProvider theme={theme}>
          <CssBaseline />
          {showAlert && (
            <Alert
              severity="info"
              action={
                <IconButton size="small" onClick={() => setShowAlert(false)}>
                  <Close />
                </IconButton>
              }
            >
              <AlertTitle>Network maintenance</AlertTitle>
              Testnet Milhon is currently down for maintenance. You may find
              that certain features in the wallet do not work during this
              period.
            </Alert>
          )}
          <Component {...pageProps} />
        </ThemeProvider>
      </ValidatorClientContext.Provider>
    </React.Fragment>
  )
}
