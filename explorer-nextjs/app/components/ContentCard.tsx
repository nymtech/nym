import { Card, CardHeader, CardContent, Typography } from '@mui/material'
import React, { ReactEventHandler } from 'react'

type ContentCardProps = {
  title?: React.ReactNode
  subtitle?: string
  Icon?: React.ReactNode
  Action?: React.ReactNode
  errorMsg?: string
  onClick?: ReactEventHandler
}

export const ContentCard: FCWithChildren<ContentCardProps> = ({
  title,
  Icon,
  Action,
  subtitle,
  errorMsg,
  children,
  onClick,
}) => (
  <Card onClick={onClick} sx={{ height: '100%' }}>
    {title && (
      <CardHeader
        title={title || ''}
        avatar={Icon}
        action={Action}
        subheader={subtitle}
      />
    )}
    {children && <CardContent>{children}</CardContent>}
    {errorMsg && (
      <Typography variant="body2" sx={{ color: 'danger', padding: 2 }}>
        {errorMsg}
      </Typography>
    )}
  </Card>
)
