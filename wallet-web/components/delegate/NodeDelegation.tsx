import React, { useContext, useEffect } from 'react'
import { Grid, Paper } from '@material-ui/core'
import { coin, printableCoin } from '@nymproject/nym-validator-client'
import { useRouter } from 'next/router'
import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'
import { ValidatorClientContext } from '../../contexts/ValidatorClient'
import { NodeType } from '../../common/node'
import NoClientError from '../NoClientError'
import Confirmation from '../Confirmation'
import DelegateForm from './DelegateForm'
import { Coin } from '@cosmjs/launchpad'
import { UDENOM } from '../../pages/_app'
import { theme } from '../../lib/theme'
import ExecFeeNotice from '../ExecFeeNotice'

const DelegateToNode = () => {
  const router = useRouter()
  const { client } = useContext(ValidatorClientContext)

  const [isLoading, setIsLoading] = React.useState<boolean>()
  const [delegationError, setDelegationError] = React.useState(null)

  const [nodeType, setNodeType] = React.useState(NodeType.Mixnode)
  const [stakeValue, setStakeValue] = React.useState('0 PUNK')
  const [nodeIdentity, setNodeIdentity] = React.useState('')

  useEffect(() => {
    const checkClient = async () => {
      if (client === null) {
        await router.push('/')
      }
    }
    checkClient()
  }, [client])

  const getDelegationValue = (raw: string): Coin => {
    let native = printableBalanceToNative(raw)
    return coin(parseInt(native), UDENOM)
  }

  const delegateToNode = async (event) => {
    event.preventDefault()
    console.log(`DELEGATE button pressed`)

    const nodeIdentity = event.target.identity.value
    const delegationValue = getDelegationValue(event.target.amount.value)

    setNodeIdentity(nodeIdentity)
    setStakeValue(printableCoin(delegationValue))
    setIsLoading(true)

    if (nodeType == NodeType.Mixnode) {
      client
        .delegateToMixnode(nodeIdentity, delegationValue)
        .then((value) => {
          console.log('delegated to mixnode!', value)
        })
        .catch(setDelegationError)
        .finally(() => setIsLoading(false))
    } else {
      client
        .delegateToGateway(nodeIdentity, delegationValue)
        .then((value) => {
          console.log('delegated to gateway!', value)
        })
        .catch(setDelegationError)
        .finally(() => setIsLoading(false))
    }
  }

  const getDelegationContent = () => {
    // we're not signed in
    if (client === null) {
      return <NoClientError />
    }

    // we haven't clicked delegate button yet
    if (isLoading === undefined) {
      return <DelegateForm onSubmit={delegateToNode} />
    }

    // We started delegation
    return (
      <Confirmation
        isLoading={isLoading}
        error={delegationError}
        progressMessage={`Delegating stake on ${nodeType} ${nodeIdentity} is in progress...`}
        successMessage={`Successfully delegated ${stakeValue} stake on ${nodeType} ${nodeIdentity}`}
        failureMessage={`Failed to delegate to a ${nodeType}!`}
      />
    )
  }

  return (
    <Grid container spacing={2} direction="column">
      <Grid item>
        <ExecFeeNotice name={'delegating stake'} />
      </Grid>
      <Grid item>
        <Paper style={{ padding: theme.spacing(3) }}>
          {getDelegationContent()}
        </Paper>
      </Grid>
    </Grid>
  )
}

export default DelegateToNode
