import React, { createContext, useCallback, useEffect, useState } from 'react'
import { useHistory } from 'react-router-dom'
import { TClientDetails } from '../types'

type TClientContext = {
  clientDetails?: TClientDetails
  logIn: (clientDetails: TClientDetails) => void
  logOut: () => void
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const [clientDetails, setClientDetails] = useState<TClientDetails>()

  const history = useHistory()

  useEffect(() => {
    !clientDetails ? history.push('/signin') : history.push('/bond')
  }, [clientDetails])

  const logIn = (clientDetails: TClientDetails) =>
    setClientDetails(clientDetails)

  const logOut = () => setClientDetails(undefined)

  return (
    <ClientContext.Provider
      value={{
        clientDetails,
        logIn,
        logOut,
      }}
    >
      {children}
    </ClientContext.Provider>
  )
}
