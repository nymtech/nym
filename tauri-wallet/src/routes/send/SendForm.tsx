import React, { useContext } from 'react'
import { Grid, InputAdornment, TextField, Typography } from '@material-ui/core'
import { ClientContext } from '../../context/main'

export const SendForm = ({
  formData,
  updateRecipAddress,
  updateAmount,
}: {
  formData: { toAddress: string; sendAmount: string }
  updateRecipAddress: (address: string) => void
  updateAmount: (amount: string) => void
}) => {
  const { client } = useContext(ClientContext)

  return (
    <Grid container spacing={3}>
      <Grid item xs={12}>
        <TextField
          required
          variant="outlined"
          id="sender"
          name="sender"
          label="Sender address"
          fullWidth
          value={client.address}
          disabled={true}
        />
      </Grid>

      <Grid item xs={12}>
        <TextField
          required
          variant="outlined"
          id="recipient"
          name="recipient"
          label="Recipient address"
          fullWidth
          autoFocus
          value={formData.toAddress}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            updateRecipAddress(e.target.value)
          }
        />
      </Grid>
      <Grid item xs={12} sm={6}>
        <TextField
          required
          variant="outlined"
          id="amount"
          name="amount"
          label="Amount"
          fullWidth
          InputProps={{
            endAdornment: <InputAdornment position="end">punks</InputAdornment>,
          }}
          value={formData.sendAmount}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            updateAmount(e.target.value)
          }
        />
      </Grid>
    </Grid>
  )
}
