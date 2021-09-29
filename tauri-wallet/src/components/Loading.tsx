import React from 'react'
import { Box, CircularProgress } from '@material-ui/core'

type TLoading = {
  size?: 'small' | 'medium' | 'large' | 'x-large'
  Icon?: React.ReactNode
}

export const Loading: React.FC<TLoading> = ({ size = 'medium', Icon }) => {
  return (
    <Box style={{ position: 'relative', display: 'inline-flex' }}>
      <CircularProgress
        size={
          size === 'small'
            ? 24
            : size === 'large'
            ? 60
            : size === 'x-large'
            ? 72
            : 36
        }
      />
      {Icon && (
        <Box
          style={{
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
            position: 'absolute',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          {Icon}
        </Box>
      )}
    </Box>
  )
}
