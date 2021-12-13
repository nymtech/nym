import React from 'react'
import { Card, CardContent, CardHeader } from '@mui/material'
import { styled } from '@mui/material/styles'

export const NymCard: React.FC<{
  title: string | React.ReactElement
  subheader?: string
  Action?: React.ReactNode
  noPadding?: boolean
}> = ({ title, subheader, Action, noPadding, children }) => {
  return (
    <Card variant="outlined">
      <CardHeader
        title={title}
        subheader={subheader}
        data-testid={title}
        titleTypographyProps={{ variant: 'h5' }}
        subheaderTypographyProps={{ variant: 'subtitle1' }}
        action={Action}
        sx={{
          color: 'nym.background.dark',
          py: 2.5,
          px: 4,
          borderBottom: (theme) => `1px solid ${theme.palette.grey[200]}`,
        }}
      />
      {noPadding ? <CardContentNoPadding>{children}</CardContentNoPadding> : <CardContent>{children}</CardContent>}
    </Card>
  )
}

const CardContentNoPadding = styled(CardContent)(({ theme }) => ({
  background: theme.palette.grey[50],
  padding: 0,
  '&:last-child': {
    paddingBottom: 0,
  },
}))
