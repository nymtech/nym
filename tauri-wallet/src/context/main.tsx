import React, { createContext } from 'react'

type TClientContext = {
  client: { address: string; balance: string }
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const client = {
    address: 'punk1s63y29jf8f3ft64z0vh80g3c76ty8lnyr74eur',
    balance: '2000 PUNKS',
  }
  return (
    <ClientContext.Provider value={{ client }}>
      {children}
    </ClientContext.Provider>
  )
}
