import React from 'react'
import { Box, Card, CardContent, CardHeader } from '@mui/material'
import { styled } from '@mui/material/styles'

export const NymCard: React.FC<{
  title: string | React.ReactElement
  subheader?: string
  Action?: React.ReactNode
  Icon?: any
  noPadding?: boolean
}> = ({ title, subheader, Action, Icon, noPadding, children }) => {
  return (
    <Card variant="outlined" sx={{ overflow: 'auto' }}>
      <CardHeader
        sx={{ p: 3, color: 'nym.background.dark' }}
        title={
          <Box display="flex" alignItems="center">
            {Icon && <Icon sx={{ mr: 1 }} />} {title}
          </Box>
        }
        subheader={subheader}
        data-testid={title}
        titleTypographyProps={{ variant: 'h5' }}
        subheaderTypographyProps={{ variant: 'subtitle1' }}
        action={<Box sx={{ mt: 1, mr: 1 }}>{Action}</Box>}
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
