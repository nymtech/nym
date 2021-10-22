import React from 'react'
import { Card, CardContent, CardHeader, useTheme } from '@material-ui/core'

export const NymCard: React.FC<{
  title: string
  subheader?: string
  Action?: React.ReactNode
  noPadding?: boolean
}> = ({ title, subheader, Action, noPadding, children }) => {
  const theme = useTheme()
  return (
    <Card variant="outlined">
      <CardHeader
        title={title}
        subheader={subheader}
        data-testid={title}
        titleTypographyProps={{ variant: 'h5' }}
        subheaderTypographyProps={{ variant: 'subtitle1' }}
        action={Action}
        style={{
          padding: theme.spacing(2.5),
          borderBottom: `1px solid ${theme.palette.grey[200]}`,
        }}
      />
      <CardContent
        style={{
          background: theme.palette.grey[50],
          padding: noPadding ? 0 : theme.spacing(2, 5),
        }}
      >
        {children}
      </CardContent>
    </Card>
  )
}
