import React, { useEffect, useState, ChangeEvent } from 'react'
import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'
import { Coin, nativeToPrintable } from '@nymproject/nym-validator-client'
import { Alert } from '@material-ui/lab'
import Grid from '@material-ui/core/Grid'
import TextField from '@material-ui/core/TextField'
import {
  Button,
  Checkbox,
  FormControlLabel,
  InputAdornment,
} from '@material-ui/core'
import bs58 from 'bs58'
import semver from 'semver'
import { NodeType } from '../../common/node'
import { theme } from '../../lib/theme'
import {
  basicRawCoinValueValidation,
  checkAllocationSize,
  makeBasicStyle,
  validateRawPort,
} from '../../common/helpers'
import { DENOM } from '../../pages/_app'
import { BondingInformation } from './NodeBond'
import { useGetBalance } from '../../hooks/useGetBalance'

const DEFAULT_MIX_PORT = 1789
const DEFAULT_VERLOC_PORT = 1790
const DEFAULT_HTTP_API_PORT = 8000
const DEFAULT_CLIENTS_PORT = 9000

type TBondNodeFormProps = {
  type: NodeType
  minimumMixnodeBond: Coin
  minimumGatewayBond: Coin
  onSubmit: (event: any) => void
}

type TFormInput = {
  amount: string
  host: string
  http_api_port: string
  mixPort: string
  verlocPort: string
  sphinxKey: string
  identityKey: string
  version: string
  location?: string
  clientsPort?: string
  httpApiPort?: string
}

export default function BondNodeForm(props: TBondNodeFormProps) {
  const classes = makeBasicStyle(theme)

  const [validity, setValidity] = React.useState({
    validAmount: true,
    validSphinxKey: true,
    validIdentityKey: true,
    validHost: true,
    validVersion: true,
    validLocation: true,
    validMixPort: true,

    // this should have probably be somehow split to be subclasses of the validity matrix
    // the above is more true now as more fields are added. This looks kinda disgusting...
    // mixnode-specific:
    validVerlocPort: true,
    validHttpApiPort: true,

    // gateway-specific:
    validClientsPort: true,
  })

  const [advancedShown, setAdvancedShown] = React.useState(false)
  const [allocationWarning, setAllocationWarning] = useState<string>()
  const [isValidAmount, setIsValidAmount] = useState(true)
  const { getBalance, accountBalance } = useGetBalance()

  useEffect(() => {
    getBalance()
  }, [getBalance])

  const handleCheckboxToggle = () => {
    setAdvancedShown((prevSet) => !prevSet)
  }

  const handleAmountChange = (e: ChangeEvent<HTMLInputElement>) => {
    const parsed = +e.target.value
    const balance = +accountBalance.amount

    if (isNaN(parsed)) {
      setIsValidAmount(false)
    } else {
      try {
        const allocationCheck = checkAllocationSize(
          +printableBalanceToNative(e.target.value),
          balance,
          'bond'
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

  const validateForm = ({
    amount,
    sphinxKey,
    identityKey,
    host,
    version,
    verlocPort,
    location,
    mixPort,
    httpApiPort,
    clientsPort,
  }: TFormInput): boolean => {
    let validAmount = validateAmount(amount)
    let validSphinxKey = validateKey(sphinxKey)
    let validIdentityKey = validateKey(identityKey)
    let validHost = validateHost(host)
    let validVersion = validateVersion(version)

    let validLocation =
      props.type == NodeType.Gateway ? validateLocation(location) : true

    let newValidity = {
      validAmount: validAmount,
      validSphinxKey: validSphinxKey,
      validIdentityKey: validIdentityKey,
      validHost: validHost,
      validVersion: validVersion,
      validLocation: validLocation,
    }

    if (advancedShown) {
      let validMixPort = validateRawPort(mixPort)
      let validVerlocPort =
        props.type == NodeType.Mixnode ? validateRawPort(verlocPort) : true
      let validHttpApiPort =
        props.type == NodeType.Mixnode ? validateRawPort(httpApiPort) : true
      let validClientsPort =
        props.type == NodeType.Gateway ? validateRawPort(clientsPort) : true

      newValidity = {
        ...newValidity,
        ...{
          validMixPort: validMixPort,
          validVerlocPort: validVerlocPort,
          validHttpApiPort: validHttpApiPort,
          validClientsPort: validClientsPort,
        },
      }
    }

    setValidity((previousState) => {
      return { ...previousState, ...newValidity }
    })

    // just AND everything together
    const reducer = (acc, current) => acc && current
    return Object.entries(newValidity)
      .map((entry) => entry[1])
      .reduce(reducer, true)
  }

  const validateAmount = (rawValue: string): boolean => {
    // tests basic coin value requirements, like no more than 6 decimal places, value lower than total supply, etc
    if (!basicRawCoinValueValidation(rawValue)) {
      return false
    }

    // this conversion seems really iffy but I'm not sure how to better approach it
    let nativeValueString = printableBalanceToNative(rawValue)
    let nativeValue = parseInt(nativeValueString)
    if (props.type == NodeType.Mixnode) {
      return nativeValue >= parseInt(props.minimumMixnodeBond.amount)
    } else {
      return nativeValue >= parseInt(props.minimumGatewayBond.amount)
    }
  }

  const validateKey = (key: string): boolean => {
    // it must be a valid base58 key
    try {
      const bytes = bs58.decode(key)
      // of length 32
      return bytes.length === 32
    } catch {
      return false
    }
  }

  const validateHost = (host: string): boolean => {
    // I don't think that proper checks are in scope of the change here
    // what would need to be checked is whether one of the following is true:
    // - host is an ipv4 address
    // - host is an ipv6 address
    // - host is a valid hostname

    // so at least perform the dumbest possible checks
    // ipv4 needs 4 dot-separated octets
    // ipv6 can have multiple possible representations, but it needs to contain at least two colons
    // a hostname (in this case) needs to have a top level domain present

    const dot_occurrences = host.trim().split('.').length - 1
    const colon_occurrences = host.trim().split(':').length - 1

    if (dot_occurrences == 3) {
      // possible ipv4
      // make sure it has no ports attached!
      return colon_occurrences == 0
    } else if (colon_occurrences >= 2) {
      // possible ipv6
      return true
    } else if (dot_occurrences >= 1) {
      // possible hostname
      // make sure it has no ports attached!
      return colon_occurrences == 0
    }
    return false
  }

  const validateVersion = (version: string): boolean => {
    // check if its a valid semver
    return semver.valid(version) && semver.minor(version) >= 11
  }

  const validateLocation = (location: string): boolean => {
    // right now only perform the stupid check of whether the user copy-pasted the tooltip... (with or without brackets)
    return !location.trim().includes('physical location of your node')
  }

  const constructMixnodeBondingInfo = ({
    amount,
    host,
    httpApiPort,
    mixPort,
    verlocPort,
    sphinxKey,
    identityKey,
    version,
  }: TFormInput): BondingInformation => {
    return {
      amount,
      nodeDetails: {
        host,
        http_api_port: advancedShown
          ? parseInt(httpApiPort)
          : DEFAULT_HTTP_API_PORT,
        mix_port: advancedShown ? parseInt(mixPort) : DEFAULT_MIX_PORT,
        verloc_port: advancedShown ? parseInt(verlocPort) : DEFAULT_VERLOC_PORT,
        sphinx_key: sphinxKey,
        identity_key: identityKey,
        version,
      },
    }
  }

  const constructGatewayBondingInfo = (
    data: TFormInput
  ): BondingInformation => {
    return {
      amount: data.amount,
      nodeDetails: {
        host: data.host,
        mix_port: advancedShown ? parseInt(data.mixPort) : DEFAULT_MIX_PORT,
        clients_port: advancedShown
          ? parseInt(data.clientsPort)
          : DEFAULT_CLIENTS_PORT,
        sphinx_key: data.sphinxKey,
        identity_key: data.identityKey,
        version: data.version,
        location: data.location,
      },
    }
  }

  const submitForm = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const target = event.target as unknown as TFormInput

    if (validateForm(target)) {
      if (props.type == NodeType.Mixnode) {
        return props.onSubmit(constructMixnodeBondingInfo(target))
      } else {
        return props.onSubmit(constructGatewayBondingInfo(target))
      }
    }
  }

  let minimumBond = props.minimumMixnodeBond
  if (props.type == NodeType.Gateway) {
    minimumBond = props.minimumGatewayBond
  }

  // if this whole interface wasn't to be completely redone in a month time, I would have definitely redone the form
  // but I guess it's fine for time being
  return (
    <form onSubmit={submitForm}>
      <Grid container spacing={3}>
        <Grid item xs={12} sm={8}>
          <TextField
            required
            id='amount'
            name='amount'
            label={`Amount to bond (minimum ${nativeToPrintable(
              minimumBond.amount
            )} ${minimumBond.denom})`}
            error={!validateAmount}
            helperText={
              !validateAmount
                ? `Enter a valid bond amount (minimum ${nativeToPrintable(
                    minimumBond.amount
                  )})`
                : ''
            }
            fullWidth
            InputProps={{
              endAdornment: (
                <InputAdornment position='end'>{DENOM}</InputAdornment>
              ),
            }}
            onChange={handleAmountChange}
          />
        </Grid>
        {allocationWarning && (
          <Grid item>
            <Alert severity={!isValidAmount ? 'error' : 'info'}>
              {allocationWarning}
            </Alert>
          </Grid>
        )}
        <Grid item xs={12}>
          <TextField
            error={!validity.validIdentityKey}
            required
            id='identity'
            name='identity'
            label='Identity key'
            fullWidth
          />
        </Grid>
        <Grid item xs={12}>
          <TextField
            error={!validity.validSphinxKey}
            required
            id='sphinxKey'
            name='sphinxKey'
            label='Sphinx key'
            fullWidth
            {...(!validity.validSphinxKey
              ? { helperText: 'Enter a valid sphinx key' }
              : {})}
          />
        </Grid>
        <Grid item xs={12} sm={6}>
          <TextField
            error={!validity.validHost}
            required
            id='host'
            name='host'
            label='Host'
            fullWidth
            {...(!validity.validHost
              ? { helperText: 'Enter a valid IP or a hostname (without port)' }
              : {})}
          />
        </Grid>

        {/* if it's a gateway - get location */}
        <Grid item xs={12} sm={6}>
          {props.type === NodeType.Gateway && (
            <TextField
              error={!validity.validLocation}
              required
              id='location'
              name='location'
              label='Location'
              fullWidth
              {...(!validity.validLocation
                ? { helperText: 'Enter a valid location of your node' }
                : {})}
            />
          )}
        </Grid>

        <Grid item xs={12} sm={6}>
          <TextField
            error={!validity.validVersion}
            required
            id='version'
            name='version'
            label='Version'
            fullWidth
            {...(!validity.validVersion
              ? {
                  helperText:
                    'Enter a valid version (min. 0.11.0), like 0.11.0',
                }
              : {})}
          />
        </Grid>

        <Grid item xs={12}>
          <FormControlLabel
            control={
              <Checkbox
                checked={advancedShown}
                onChange={handleCheckboxToggle}
              />
            }
            label='Show advanced options'
          />
        </Grid>

        {advancedShown && (
          <>
            <Grid item xs={12} sm={4}>
              <TextField
                error={!validity.validMixPort}
                variant='outlined'
                id='mixPort'
                name='mixPort'
                label='Mix Port'
                fullWidth
                defaultValue={DEFAULT_MIX_PORT}
                {...(!validity.validMixPort
                  ? { helperText: 'Enter a valid version, like 0.10.0' }
                  : {})}
              />
            </Grid>

            {/*yes, I also hate so many layers of indentation here*/}
            {props.type === NodeType.Mixnode ? (
              <>
                <Grid item xs={12} sm={4}>
                  <TextField
                    error={!validity.validVerlocPort}
                    variant='outlined'
                    id='verlocPort'
                    name='verlocPort'
                    label='Verloc Port'
                    fullWidth
                    defaultValue={DEFAULT_VERLOC_PORT}
                  />
                </Grid>

                <Grid item xs={12} sm={4}>
                  <TextField
                    error={!validity.validHttpApiPort}
                    variant='outlined'
                    id='httpApiPort'
                    name='httpApiPort'
                    label='HTTP API Port'
                    fullWidth
                    defaultValue={DEFAULT_HTTP_API_PORT}
                  />
                </Grid>
              </>
            ) : (
              <Grid item xs={12} sm={4}>
                <TextField
                  error={!validity.validClientsPort}
                  variant='outlined'
                  id='clientsPort'
                  name='clientsPort'
                  label='client WS API Port'
                  fullWidth
                  defaultValue={DEFAULT_CLIENTS_PORT}
                />
              </Grid>
            )}
          </>
        )}
      </Grid>

      <div className={classes.buttons}>
        <Button
          variant='contained'
          color='primary'
          type='submit'
          className={classes.button}
          disabled={!isValidAmount}
        >
          Bond
        </Button>
      </div>
    </form>
  )
}
