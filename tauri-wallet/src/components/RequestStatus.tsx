import React from 'react'
import { CircularProgress, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'

export enum EnumRequestStatus {
  initial,
  error,
  loading,
  success,
}

export const RequestStatus = ({
  status,
  onSuccess,
  onError,
}: {
  status: EnumRequestStatus
  onSuccess: () => void
  onError: () => void
}) => {
  const theme: Theme = useTheme()
  return (
    <div style={{ padding: theme.spacing(3, 5) }}>
      {status === EnumRequestStatus.loading && (
        <div style={{ display: 'flex', justifyContent: 'center' }}>
          <CircularProgress size={48} />
        </div>
      )}
      {status === EnumRequestStatus.success && onSuccess()}
      {status === EnumRequestStatus.error && onError()}
    </div>
  )
}
