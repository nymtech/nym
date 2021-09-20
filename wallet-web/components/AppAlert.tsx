import { IconButton } from '@material-ui/core'
import { Close } from '@material-ui/icons'
import { Alert, AlertProps, AlertTitle } from '@material-ui/lab'
import React, { useState } from 'react'

export const AppAlert = ({
  message,
  severity = 'info',
  title,
}: {
  message: string
  severity?: AlertProps['severity']
  title?: string
}) => {
  const [showAlert, setShowAlert] = useState(true)

  return showAlert ? (
    <Alert
      severity={severity}
      action={
        <IconButton size="small" onClick={() => setShowAlert(false)}>
          <Close />
        </IconButton>
      }
    >
      <AlertTitle>{title}</AlertTitle>
      {message}
    </Alert>
  ) : null
}
