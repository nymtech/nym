import React from 'react'
import { Box } from '@mui/material'

export const PageLayout: React.FC<{ position?: 'flex-start' | 'flex-end' }> = ({ position, children }) => {
  return (
    <Box
      sx={{
        height: 'calc(100% - 65px)',
        display: 'flex',
        alignItems: position || 'center',
        overflow: 'auto',
      }}
    >
      <Box width="100%" margin="auto">
        {children}
      </Box>
    </Box>
  )
}
