import React from 'react'
import { Box } from '@mui/material'
import { Nav } from '../components'
import Logo from '../images/logo-background.svg'
import { AppBar } from '../components'

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
          py: 4,
          px: 5,
        }}
        display="flex"
        flexDirection="column"
        justifyContent="space-between"
      >
        <Box>
          <Box sx={{ mb: 3 }}>
            <Logo width={45} />
          </Box>
          <Nav />
        </Box>
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
