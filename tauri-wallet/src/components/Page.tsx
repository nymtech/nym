import React from 'react'
import { BalanceCard } from './BalanceCard'
import { Nav } from './Nav'

export const Page = ({ children }: { children: React.ReactElement }) => {
  return (
    <div
      style={{
        height: '100vh',
        width: '100vw',
        display: 'grid',
        gridTemplateColumns: '400px auto',
        gridTemplateRows: '100%',
        gridColumnGap: '8px',
        gridRowGap: '0px',
      }}
    >
      <div
        style={{
          gridArea: '1 / 1 / 2 / 2',
          background: '#121726',

          borderTopRightRadius: 10,
          borderBottomRightRadius: 10,
        }}
      >
        <BalanceCard />
        <Nav />
        <div />
      </div>
      <div
        style={{
          gridArea: '1 / 2 / 2 / 3',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {children}
      </div>
    </div>
  )
}
