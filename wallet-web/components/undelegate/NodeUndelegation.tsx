import React, { useContext, useEffect } from 'react'
import { Grid, Paper } from '@material-ui/core'
import { useRouter } from 'next/router'
import { ValidatorClientContext } from '../../contexts/ValidatorClient'
import { NodeType } from '../../common/node'
import NoClientError from '../NoClientError'
import Confirmation from '../Confirmation'
import { theme } from '../../lib/theme'
import NodeTypeChooser from '../NodeTypeChooser'
import NodeIdentityForm from '../NodeIdentityForm'
import ExecFeeNotice from '../ExecFeeNotice'

const UndelegateFromNode = () => {
  const router = useRouter()
  const { client } = useContext(ValidatorClientContext)

  const [isLoading, setIsLoading] = React.useState<boolean>()
  const [undelegationError, setUndelegationError] = React.useState(null)

  const [nodeType, setNodeType] = React.useState(NodeType.Mixnode)

  useEffect(() => {
    const checkClient = async () => {
      if (client === null) {
        await router.push('/')
      }
    }
    checkClient()
  }, [client])

  const undelegateFromNode = async (event) => {
    event.preventDefault()

    console.log(`UNDELEGATE button pressed`)

    let address = event.target.identity.value
    setIsLoading(true)

    if (nodeType == NodeType.Mixnode) {
      client
        .removeMixnodeDelegation(address)
        .then((value) => {
          console.log('undelegated from mixnode!', value)
        })
        .catch(setUndelegationError)
        .finally(() => setIsLoading(false))
    } else {
      client
        .removeGatewayDelegation(address)
        .then((value) => {
          console.log('undelegated from gateway!', value)
        })
        .catch(setUndelegationError)
        .finally(() => setIsLoading(false))
    }
  }

  const getUndelegationContent = () => {
    // we're not signed in
    if (client === null) {
      return <NoClientError />
    }

    // we haven't clicked undelegate button yet
    if (isLoading === undefined) {
      return (
        <NodeIdentityForm
          onSubmit={undelegateFromNode}
          buttonText={'Remove delegation'}
        />
      )
    }

    // We started delegation
    return (
      <Confirmation
        isLoading={isLoading}
        error={undelegationError}
        progressMessage={`${nodeType} undelegation is in progress...`}
        successMessage={`${nodeType} undelegation was successful!`}
        failureMessage={`Failed to undelegate from a ${nodeType}!`}
      />
    )
  }

  return (
    <Grid container spacing={2} direction="column">
      <Grid item>
        <ExecFeeNotice name={'undelegating stake'} />
      </Grid>
      <Grid item>
        <Paper style={{ padding: theme.spacing(3) }}>
          {getUndelegationContent()}
        </Paper>
      </Grid>
    </Grid>
  )
}

export default UndelegateFromNode
