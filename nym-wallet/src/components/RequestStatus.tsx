import React from 'react'
import { CircularProgress, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'

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
  const theme: Theme = useTheme()
  return (
    <div style={{ padding: theme.spacing(3, 5) }}>
      {status === EnumRequestStatus.loading && (
        <div style={{ display: 'flex', justifyContent: 'center' }}>
          <CircularProgress size={48} />
        </div>
      )}
      {status === EnumRequestStatus.success && Success}
      {status === EnumRequestStatus.error && Error}
    </div>
  )
}
