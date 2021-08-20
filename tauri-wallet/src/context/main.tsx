import React, { createContext } from 'react'

type TClientContext = {
  client: {}
}

export const ClientContext = createContext({} as TClientContext)

export const ClientContextProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const client = {}
  return (
    <ClientContext.Provider value={{ client }}>
      {children}
    </ClientContext.Provider>
  )
}
