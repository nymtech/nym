import React from 'react'
import { CircularProgress, Box } from '@mui/material'

export enum EnumRequestStatus {
  initial = 'initial',
  error = 'error',
  loading = 'loading',
  success = 'success',
}

export const RequestStatus = ({
  status,
  Success,
  Error,
}: {
  status: EnumRequestStatus
  Success: React.ReactNode
  Error: React.ReactNode
}) => {
  return (
    <Box sx={{ padding: [3, 5] }}>
      {status === EnumRequestStatus.loading && (
        <Box sx={{ display: 'flex', justifyContent: 'center' }}>
          <CircularProgress size={48} />
        </Box>
      )}
      {status === EnumRequestStatus.success && Success}
      {status === EnumRequestStatus.error && Error}
    </Box>
  )
}
