import React, { useState, useEffect } from 'react'
import Grid from '@material-ui/core/Grid'
import { Button, InputAdornment } from '@material-ui/core'
import TextField from '@material-ui/core/TextField'
import { Alert } from '@material-ui/lab'
import { DENOM } from '../../pages/_app'
import { theme } from '../../lib/theme'
import {
  basicRawCoinValueValidation,
  checkAllocationSize,
  makeBasicStyle,
  validateIdentityKey,
} from '../../common/helpers'
import { useGetBalance } from '../../hooks/useGetBalance'
import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'

type DelegateFormProps = {
  onSubmit: (event: any) => void
}

export default function DelegateForm(props: DelegateFormProps) {
  const classes = makeBasicStyle(theme)

  const [isValidAmount, setIsValidAmount] = useState(true)
  const [validIdentity, setValidIdentity] = useState(true)
  const [allocationWarning, setAllocationWarning] = useState<string>()
  const { getBalance, accountBalance } = useGetBalance()

  useEffect(() => {
    getBalance()
  }, [getBalance])

  const handleAmountChange = (event: any) => {
    // don't ask me about that. javascript works in mysterious ways
    // and this is apparently a good way of checking if string
    // is purely made of numeric characters
    const parsed = +event.target.value
    const balance = +accountBalance.amount

    if (isNaN(parsed)) {
      setIsValidAmount(false)
    } else {
      try {
        const allocationCheck = checkAllocationSize(
          +printableBalanceToNative(event.target.value),
          balance,
          'delegate'
        )
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

  const validateForm = (event: any): boolean => {
    let validIdentity = validateIdentityKey(event.target.identity.value)
    let validAmount = validateAmount(event.target.amount.value)

    setValidIdentity(validIdentity)
    setIsValidAmount(validAmount)

    return validIdentity && validAmount
  }

  const validateAmount = (rawAmount: string): boolean => {
    return basicRawCoinValueValidation(rawAmount)
  }

  const submitForm = (event: any) => {
    event.preventDefault()

    if (validateForm(event)) {
      return props.onSubmit(event)
    }
  }

  return (
    <form onSubmit={submitForm}>
      <Grid container spacing={3} direction="column">
        <Grid item xs={12}>
          <TextField
            required
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

        <Grid item xs={12} lg={6}>
          <TextField
            required
            id="amount"
            name="amount"
            label="Amount to delegate"
            error={!isValidAmount}
            helperText={isValidAmount ? '' : 'Please enter a valid amount'}
            onChange={handleAmountChange}
            fullWidth
            InputProps={{
              endAdornment: (
                <InputAdornment position="end">{DENOM}</InputAdornment>
              ),
            }}
          />
        </Grid>
        {allocationWarning && (
          <Grid item xs={12} lg={6}>
            <Alert severity={!isValidAmount ? 'error' : 'info'}>
              {allocationWarning}
            </Alert>
          </Grid>
        )}

        {/*<Grid item xs={12}>*/}
        {/*    <FormControlLabel*/}
        {/*        control={*/}
        {/*            <Checkbox*/}
        {/*                checked={checkboxSet}*/}
        {/*                onChange={handleCheckboxToggle}*/}

        {/*            />*/}
        {/*        }*/}
        {/*        label="checkbox text"*/}
        {/*    />*/}
        {/*</Grid>*/}
      </Grid>
      <div className={classes.buttons}>
        <Button
          variant="contained"
          color="primary"
          type="submit"
          className={classes.button}
          disabled={!isValidAmount}
        >
          Delegate stake
        </Button>
      </div>
    </form>
  )
}
