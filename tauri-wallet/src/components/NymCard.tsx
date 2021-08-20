import React from 'react'
import { Card, CardContent, CardHeader, useTheme } from '@material-ui/core'

export const NymCard = ({
  title,
  subheader,
  children,
}: {
  title: string
  subheader?: string
  children: React.ReactElement
}) => {
  const theme = useTheme()
  return (
    <Card variant="outlined">
      <CardHeader
        title={title}
        subheader={subheader}
        titleTypographyProps={{ variant: 'h5' }}
        subheaderTypographyProps={{ variant: 'subtitle1' }}
        style={{
          padding: theme.spacing(2.5),
          borderBottom: `1px solid ${theme.palette.grey[200]}`,
        }}
      />
      <CardContent
        style={{
          background: theme.palette.grey[50],
          padding: theme.spacing(2, 5),
        }}
      >
        {children}
      </CardContent>
    </Card>
  )
}
