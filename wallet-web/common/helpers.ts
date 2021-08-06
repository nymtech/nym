import { createStyles, makeStyles, Theme } from '@material-ui/core/styles'
import ValidatorClient, {
  nymGasLimits,
  nymGasPrice,
  printableCoin,
} from '@nymproject/nym-validator-client'
import { ADDRESS_LENGTH, DENOM, KEY_LENGTH, UDENOM } from '../pages/_app'
import { buildFeeTable } from '@cosmjs/launchpad'
import bs58 from 'bs58'

export const makeBasicStyle = makeStyles((theme: Theme) =>
  createStyles({
    appBar: {
      position: 'relative',
    },
    root: {
      textAlign: 'center',
      paddingTop: theme.spacing(4),
    },
    layout: {
      width: 'auto',
      marginLeft: theme.spacing(2),
      marginRight: theme.spacing(2),
      [theme.breakpoints.up(600 + theme.spacing(2) * 2)]: {
        width: 650,
        marginLeft: 'auto',
        marginRight: 'auto',
      },
    },
    paper: {
      marginTop: theme.spacing(3),
      marginBottom: theme.spacing(3),
      padding: theme.spacing(2),
      [theme.breakpoints.up(600 + theme.spacing(3) * 2)]: {
        marginTop: theme.spacing(6),
        marginBottom: theme.spacing(6),
        padding: theme.spacing(3),
      },
    },
    stepper: {
      padding: theme.spacing(3, 0, 5),
    },
    buttons: {
      display: 'flex',
      justifyContent: 'flex-end',
    },
    button: {
      marginTop: theme.spacing(3),
      marginLeft: theme.spacing(1),
    },
    menuButton: {
      marginRight: theme.spacing(2),
    },
    list: {
      width: 250,
    },
    wrapper: {
      marginTop: theme.spacing(1),
      marginBottom: theme.spacing(3),
    },
  })
)

type NodeOwnership = {
  ownsMixnode: boolean
  ownsGateway: boolean
}

export async function checkNodesOwnership(
  client: ValidatorClient
): Promise<NodeOwnership> {
  const ownsMixnodePromise = client.ownsMixNode()
  const ownsGatewayPromise = client.ownsGateway()

  let ownsMixnode = false
  let ownsGateway = false

  await Promise.allSettled([ownsMixnodePromise, ownsGatewayPromise]).then(
    (results) => {
      if (results[0].status === 'fulfilled') {
        ownsMixnode = results[0].value
      } else {
        console.error('failed to check for mixnode ownership')
      }
      if (results[1].status === 'fulfilled') {
        ownsGateway = results[1].value
      } else {
        console.error('failed to check for gateway ownership')
      }
    }
  )

  return {
    ownsMixnode,
    ownsGateway,
  }
}

export const validateClientAddress = (address: string): boolean => {
  return address.length === ADDRESS_LENGTH && address.startsWith(DENOM)
}

export const validateIdentityKey = (key: string): boolean => {
  try {
    const bytes = bs58.decode(key)
    // of length 32
    return bytes.length === KEY_LENGTH
  } catch {
    return false
  }
}

export const validateRawPort = (rawPort: number): boolean =>
  !isNaN(rawPort) && rawPort >= 1 && rawPort <= 65535
// first of all it must be an integer
// and it must be a non-zero 16 bit unsigned integer

export const basicRawCoinValueValidation = (rawAmount: string): boolean => {
  let amountFloat = parseFloat(rawAmount)
  if (isNaN(amountFloat)) {
    return false
  }

  // it cannot have more than 6 decimal places
  if (amountFloat != parseFloat(amountFloat.toFixed(6))) {
    return false
  }

  // it cannot be larger than the total supply
  if (amountFloat > 1_000_000_000_000_000) {
    return false
  }

  // it can't be lower than one micro coin
  return amountFloat >= 0.000001
}

export const getDisplayExecGasFee = (): string => {
  const table = buildFeeTable(nymGasPrice(DENOM), nymGasLimits, nymGasLimits)
  return printableCoin(table.exec.amount[0])
}

export const getDisplaySendGasFee = (): string => {
  const table = buildFeeTable(nymGasPrice(DENOM), nymGasLimits, nymGasLimits)
  return printableCoin(table.send.amount[0])
}

// Check amount to bond or delegate is valid
export const checkAllocationSize = (
  allocationValue: number,
  walletValue: number,
  transactionType: 'bond' | 'delegate'
) => {
  const remaining = walletValue - allocationValue
  const table = buildFeeTable(nymGasPrice(DENOM), nymGasLimits, nymGasLimits)
  const threshold =
    transactionType === 'bond'
      ? +table.exec.amount[0].amount * 2
      : +table.exec.amount[0].amount

  if (remaining < 0) {
    return {
      error: true,
      message: 'The allocation size is greater than the value of your wallet',
    }
  }

  if (walletValue > 0 && remaining < threshold) {
    return {
      error: false,
      message: `You'll only have ${printableCoin({
        amount: remaining.toString(),
        denom: UDENOM,
      })} after this transaction. You may want to keep some in order to un${transactionType} this mixnode at a later time.`,
    }
  }

  return {
    error: false,
    message: undefined,
  }
}
