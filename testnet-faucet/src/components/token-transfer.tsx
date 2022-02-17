import { useContext } from 'react'
import { Card, CardHeader, Link, Typography } from '@mui/material'
import { GlobalContext, urls } from '../context/index'

export const TokenTransferComplete = () => {
  const { tokenTransfer } = useContext(GlobalContext)

  if (tokenTransfer) {
    return (
      <Card
        sx={{
          background: 'transparent',
          border: (theme) => `1px solid ${theme.palette.common.white}`,
          p: 2,
          overflow: 'auto',
        }}
      >
        <CardHeader
          title={
            <>
              <Typography component="span" variant="h5">
                Successfully transferred {tokenTransfer.amount} NYMT to
              </Typography>{' '}
              <Link
                target="_blank"
                rel="noopener"
                href={`${urls.blockExplorer}/account/${tokenTransfer.address}`}
                data-testid="success-sent-message"
                variant="h5"
              >
                {tokenTransfer.address}
              </Link>
            </>
          }
        />
      </Card>
    )
  }
  return null
}
