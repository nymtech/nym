import React from 'react'
import { Button, Stack } from '@mui/material'
import { SubtitleSlick, Title } from '../components'

export const WelcomeContent = ({
  onUseExisting,
  onCreateAccountComplete,
}: {
  page: 'welcome'
  onUseExisting: () => void
  onCreateAccountComplete: () => void
}) => {
  return (
    <>
      <Title title="Welcome to NYM" />
      <SubtitleSlick subtitle="Next generation of privacy" />
      <Stack spacing={3} sx={{ width: 300 }}>
        <Button
          fullWidth
          variant="contained"
          color="primary"
          disableElevation
          size="large"
          onClick={onCreateAccountComplete}
        >
          Create Account
        </Button>
        <Button
          fullWidth
          variant="outlined"
          size="large"
          sx={{ color: 'common.white', border: '1px solid white', '&:hover': { border: '1px solid white' } }}
          onClick={onUseExisting}
          disableRipple
        >
          Use Existing Account
        </Button>
      </Stack>
    </>
  )
}
