import React, { useState } from 'react'
import { Alert } from '@material-ui/lab'
import { Button, Grid, TextField, Theme } from '@material-ui/core'
import { useGetBalance } from '../../hooks/useGetBalance'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import { EnumNodeType } from '../../types/global'
import { useTheme } from '@material-ui/styles'

export const UndelegateForm = () => {
  const [isValidAmount, setIsValidAmount] = useState(true)
  const [validIdentity, setValidIdentity] = useState(true)
  const [allocationWarning, setAllocationWarning] = useState<string>()
  const [nodeType, setNodeType] = useState(EnumNodeType.Mixnode)

  const { getBalance, accountBalance } = useGetBalance()
  const theme: Theme = useTheme()

  const handleAmountChange = (event: any) => {
    // don't ask me about that. javascript works in mysterious ways
    // and this is apparently a good way of checking if string
    // is purely made of numeric characters
    const parsed = +event.target.value

    if (isNaN(parsed)) {
      setIsValidAmount(false)
    } else {
      try {
        const allocationCheck = { error: undefined, message: '' }
        if (allocationCheck.error) {
          setAllocationWarning(allocationCheck.message)
          setIsValidAmount(false)
        } else {
          setAllocationWarning(allocationCheck.message)
          setIsValidAmount(true)
        }
      } catch {
        setIsValidAmount(false)
      }
    }
  }

  return (
    <form onSubmit={() => {}}>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3} direction="column">
          <Grid item xs={12}>
            <NodeTypeSelector
              nodeType={nodeType}
              setNodeType={(nodeType) => setNodeType(nodeType)}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              required
              variant="outlined"
              id="identity"
              name="identity"
              label="Node identity"
              error={!validIdentity}
              helperText={
                validIdentity
                  ? ''
                  : "Please enter a valid identity like '824WyExLUWvLE2mpSHBatN4AoByuLzfnHFeHWiBYzg4z'"
              }
              fullWidth
            />
          </Grid>

          {allocationWarning && (
            <Grid item xs={12} lg={6}>
              <Alert severity={!isValidAmount ? 'error' : 'info'}>
                {allocationWarning}
              </Alert>
            </Grid>
          )}
        </Grid>
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: `1px solid ${theme.palette.grey[200]}`,
          background: theme.palette.grey[100],
          padding: theme.spacing(2),
        }}
      >
        <Button
          variant="contained"
          color="primary"
          type="submit"
          disabled={!isValidAmount}
          disableElevation
        >
          Undelegate stake
        </Button>
      </div>
    </form>
  )
}
