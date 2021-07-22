import React, { useContext, useEffect } from 'react'
import { Paper } from '@material-ui/core'
import Typography from '@material-ui/core/Typography'
import { useRouter } from 'next/router'
import { ValidatorClientContext } from '../../contexts/ValidatorClient'
import { NodeType } from '../../common/node'
import NoClientError from '../NoClientError'
import Confirmation from '../Confirmation'
import DelegateForm from './DelegateForm'
import { printableBalanceToNative } from '@nymproject/nym-validator-client/dist/currency'
import { Coin } from '@cosmjs/launchpad'
import { coin, printableCoin } from '@nymproject/nym-validator-client'
import { UDENOM } from '../../pages/_app'
import { theme } from '../../lib/theme'
import { makeBasicStyle } from '../../common/helpers'
import NodeTypeChooser from '../NodeTypeChooser'
import ExecFeeNotice from '../ExecFeeNotice'

const DelegateToNode = () => {
  const classes = makeBasicStyle(theme)
  const router = useRouter()
  const { client } = useContext(ValidatorClientContext)

  const [delegationStarted, setDelegationStarted] = React.useState(false)
  const [delegationFinished, setDelegationFinished] = React.useState(false)
  const [delegationError, setDelegationError] = React.useState(null)

  const [nodeType, setNodeType] = React.useState(NodeType.Mixnode)
  const [stakeValue, setStakeValue] = React.useState('0 HAL')
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
    setDelegationStarted(true)

    if (nodeType == NodeType.Mixnode) {
      client
        .delegateToMixnode(nodeIdentity, delegationValue)
        .then((value) => {
          console.log('delegated to mixnode!', value)
        })
        .catch(setDelegationError)
        .finally(() => setDelegationFinished(true))
    } else {
      client
        .delegateToGateway(nodeIdentity, delegationValue)
        .then((value) => {
          console.log('delegated to gateway!', value)
        })
        .catch(setDelegationError)
        .finally(() => setDelegationFinished(true))
    }
  }

  const getDelegationContent = () => {
    // we're not signed in
    if (client === null) {
      return <NoClientError />
    }

    // we haven't clicked delegate button yet
    if (!delegationStarted) {
      return (
        <>
          <NodeTypeChooser nodeType={nodeType} setNodeType={setNodeType} />
          <DelegateForm onSubmit={delegateToNode} />
        </>
      )
    }

    // We started delegation
    return (
      <Confirmation
        finished={delegationFinished}
        error={delegationError}
        progressMessage={`Delegating stake on ${nodeType} ${nodeIdentity} is in progress...`}
        successMessage={`Successfully delegated ${stakeValue} stake on ${nodeType} ${nodeIdentity}`}
        failureMessage={`Failed to delegate to a ${nodeType}!`}
      />
    )
  }

  return (
    <>
      <main className={classes.layout}>
        <Paper className={classes.paper}>
          <ExecFeeNotice name={'delegating stake'} />
          <Typography
            component='h1'
            variant='h4'
            align='center'
            className={classes.wrapper}
          >
            Delegate to {nodeType}
          </Typography>
          {getDelegationContent()}
        </Paper>
      </main>
    </>
  )
}

export default DelegateToNode
