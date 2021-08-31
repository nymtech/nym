import React from 'react'
import { Divider } from '@material-ui/core'
import { AddressCard, BalanceCard } from './NavigationCards'
import { Nav } from './Nav'
import Logo from '../images/logo.png'
import { theme } from '../theme'

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
          overflow: 'auto',
        }}
      >
        <div
          style={{
            display: 'flex',
            justifyContent: 'center',
            marginTop: theme.spacing(6),
          }}
        >
          <img src={Logo} style={{ width: 75 }} />
        </div>
        <Divider
          light
          variant="middle"
          style={{
            background: theme.palette.grey[100],
            marginTop: theme.spacing(6),
          }}
        />
        <div style={{ marginTop: theme.spacing(10) }}>
          <BalanceCard />
          <AddressCard />
        </div>

        <div style={{ marginTop: theme.spacing(7) }}>
          <Nav />
        </div>
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
