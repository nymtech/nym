import React from 'react'
import { Box, Divider } from '@mui/material'
import { Nav } from '../components'
import Logo from '../images/logo-background.svg'

export const ApplicationLayout: React.FC = ({ children }) => {
  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'grid',
        gridTemplateColumns: '240px auto',
        gridTemplateRows: '100%',
        gridColumnGap: '8px',
        gridRowGap: '0px',
        overflow: 'hidden',
      }}
    >
      <Box
        sx={{
          gridArea: '1 / 1 / 2 / 2',
          background: '#121726',
          overflow: 'auto',
        }}
      >
        <Box sx={{ display: 'flex', justifyContent: 'center', marginTop: 6 }}>
          <img src={Logo} style={{ width: 45 }} />
        </Box>
        <Divider
          light
          variant="middle"
          sx={{ bgcolor: (theme) => theme.palette.grey[100], marginTop: 6 }}
        />

        <Box sx={{ marginTop: 7 }}>
          <Nav />
        </Box>
        <Box />
      </Box>
      <Box
        sx={{
          gridArea: '1 / 2 / 2 / 3',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {children}
      </Box>
    </Box>
  )
}
