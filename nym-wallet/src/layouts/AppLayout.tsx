import React from 'react'
import { Box, Divider } from '@mui/material'
import { Nav } from '../components'
import Logo from '../images/logo-background.svg'
import { AppBar } from '../components/AppBar'

export const ApplicationLayout: React.FC = ({ children }) => {
  return (
    <Box
      sx={{
        height: '100vh',
        width: '100vw',
        display: 'grid',
        gridTemplateColumns: '240px auto',
        gridTemplateRows: '100%',
        overflow: 'hidden',
      }}
    >
      <Box
        sx={{
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
          bgcolor: 'nym.background.light',
          overflow: 'auto',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <AppBar />
        {children}
      </Box>
    </Box>
  )
}
