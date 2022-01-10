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
    <Card variant="outlined" sx={{ overflow: 'auto' }}>
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
        }}
      />
      {noPadding ? <CardContentNoPadding>{children}</CardContentNoPadding> : <CardContent>{children}</CardContent>}
    </Card>
  )
}

const CardContentNoPadding = styled(CardContent)(({ theme }) => ({
  padding: 0,
  '&:last-child': {
    paddingBottom: 0,
  },
}))
