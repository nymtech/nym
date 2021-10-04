import React, { createContext, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'
import { useSnackbar } from 'notistack'
import { TClientDetails, TSignInWithMnemonic } from '../types'
import { TUseGetBalance, useGetBalance } from '../hooks/useGetBalance'
import { Button } from '@material-ui/core'
import { theme } from '../theme'

export const ADMIN_ADDRESS = 'punk1h3w4nj7kny5dfyjw2le4vm74z03v9vd4dstpu0'

type TClientContext = {
  clientDetails?: TClientDetails
  getBalance: TUseGetBalance
  showAdmin: boolean
  ss5IsActive: boolean
  bandwidthLimit: number
  bandwidthUsed: number
  handleSetBandwidthLimit: (bandwidth: number) => void
  toggleSs5: () => void
  handleShowAdmin: () => void
  logIn: (clientDetails: TSignInWithMnemonic) => void
  logOut: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const [clientDetails, setClientDetails] = useState<TClientDetails>()
  const [showAdmin, setShowAdmin] = useState(false)
  const [ss5IsActive, setss5IsActive] = useState(false)
  const [bandwidthLimit, setBandwidthLimit] = useState(0)
  const [bandwidthUsed, setBandwidthUsed] = useState(0)

  const history = useHistory()
  const getBalance = useGetBalance()
  const { enqueueSnackbar, closeSnackbar } = useSnackbar()

  useEffect(() => {
    !clientDetails ? history.push('/signin') : history.push('/balance')
  }, [clientDetails])

  const handleSetBandwidthLimit = (bandwidth: number) =>
    setBandwidthLimit(bandwidth)

  useEffect(() => {
    let timer

    if (ss5IsActive && bandwidthUsed < bandwidthLimit) {
      timer = setTimeout(() => {
        setBandwidthUsed((used) => used + 50)
      }, 1000)
    } else if (ss5IsActive && bandwidthUsed === bandwidthLimit) {
      setBandwidthLimit(0)
      setBandwidthUsed(0)
      setss5IsActive(false)
      enqueueSnackbar(
        "You're out of bandwidth. You'll need to purchase more to continue using Socks5",
        {
          variant: 'error',
          anchorOrigin: { horizontal: 'center', vertical: 'bottom' },
          persist: true,
          action: (key) => (
            <Button
              style={{
                color: theme.palette.common.white,
              }}
              onClick={() => closeSnackbar(key)}
            >
              Dismiss
            </Button>
          ),
        }
      )
      clearTimeout(timer)
    }
  }, [ss5IsActive, bandwidthUsed, bandwidthLimit, handleSetBandwidthLimit])

  const logIn = async (clientDetails: TSignInWithMnemonic) =>
    setClientDetails(clientDetails)

  const logOut = () => setClientDetails(undefined)

  const handleShowAdmin = () => setShowAdmin((show) => !show)

  const toggleSs5 = () => setss5IsActive((active) => !active)

  return (
    <ClientContext.Provider
      value={{
        clientDetails,
        getBalance,
        showAdmin,
        ss5IsActive,
        bandwidthLimit,
        bandwidthUsed,
        toggleSs5,
        handleSetBandwidthLimit,
        handleShowAdmin,
        logIn,
        logOut,
      }}
    >
      {children}
    </ClientContext.Provider>
  )
}
