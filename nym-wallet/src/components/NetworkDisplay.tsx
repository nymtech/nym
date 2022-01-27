import React, { useContext } from 'react'
import { Box, Typography } from '@mui/material'
import { ClientContext } from '../context/main'

export const NetworkDisplay = () => {
  const { network } = useContext(ClientContext)

  return (
    <Box>
      <Typography component="span" sx={{ color: 'common.white' }} variant="subtitle2">
        Network:{' '}
      </Typography>
      <Typography component="span" color="primary" variant="subtitle2">
        {network}
      </Typography>
    </Box>
  )
}
