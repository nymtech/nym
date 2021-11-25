import React from 'react'
import { Box, Divider } from '@mui/material'
import { AddressCard, BalanceCard, Nav } from '../components'
import Logo from '../images/logo-background.svg'

export const ApplicationLayout: React.FC = ({ children }) => {
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
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          gridArea: '1 / 1 / 2 / 2',
          background: '#121726',
          overflow: 'auto',
        }}
      >
        <Box sx={{ display: 'flex', justifyContent: 'center', marginTop: 6 }}>
          <img src={Logo} style={{ width: 75 }} />
        </Box>
        <Divider
          light
          variant="middle"
          sx={{ bgcolor: (theme) => theme.palette.grey[100], marginTop: 6 }}
        />

        <div style={{ marginTop: 7 }}>
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
