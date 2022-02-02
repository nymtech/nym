import React, { useEffect, useState } from 'react'
import { LockOutlined } from '@mui/icons-material'
import { LinearProgress, Stack, Typography } from '@mui/material'
import { Box } from '@mui/system'

type TStrength = 'weak' | 'medium' | 'strong' | 'init'

const strong = /^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#\$%\^&\*])(?=.{8,})/
const medium = /^(((?=.*[a-z])(?=.*[A-Z]))|((?=.*[a-z])(?=.*[0-9]))|((?=.*[A-Z])(?=.*[0-9])))(?=.{6,})/

const colorMap = {
  init: "inherit" as "inherit",
  weak: 'error' as 'error',
  medium: 'warning' as 'warning',
  strong: 'success' as 'success',
}

const getText = (strength: TStrength) => {
  switch (strength) {
    case 'strong':
      return 'Strong password'
    case 'medium':
      return 'Medium strength password'
    case 'weak':
      return 'Weak password'
    default:
      return 'Password strength'
  }
}

const getTextColor = (strength: TStrength) => {
  switch (strength) {
    case 'strong':
      return 'success.main'
    case 'medium':
      return 'warning.main'
    case 'weak':
      return 'error.main'
    default:
      return 'grey.500'
  }
}

export const PasswordStrength = ({ password }: { password: string }) => {
  const [strength, setStrength] = useState<TStrength>('init')


  useEffect(() => {
    if (password.length === 0) {
      return setStrength('init')
    }

    if (password.match(strong)) {
      return setStrength('strong')
    }

    if (password.match(medium)) {
      return setStrength('medium')
    }
    setStrength('weak')
  }, [password])

  return (
    <Stack spacing={0.5}>
      <LinearProgress
        variant="determinate"
        color={colorMap[strength]}
        value={strength === 'strong' ? 100 : strength === 'medium' ? 50 : 0}
      />
      <Box display="flex" alignItems="center">
        <LockOutlined sx={{ fontSize: 15,  color: getTextColor(strength) }} />
        <Typography variant="caption" sx={{ ml: 0.5, color: getTextColor(strength) }}>
          {getText(strength)}
        </Typography>
      </Box>
    </Stack>
  )
}
