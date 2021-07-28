import React, { useContext, useEffect } from 'react'
import Typography from '@material-ui/core/Typography'
import { Grid, LinearProgress, Paper } from '@material-ui/core'
import { useRouter } from 'next/router'
import { NodeType } from '../../common/node'
import { ValidatorClientContext } from '../../contexts/ValidatorClient'
import NoClientError from '../NoClientError'
import UnbondNotice from './UnbondNotice'
import Confirmation from '../Confirmation'
import { theme } from '../../lib/theme'
import { checkNodesOwnership, makeBasicStyle } from '../../common/helpers'
import ExecFeeNotice from '../ExecFeeNotice'

const UnbondNode = () => {
  const classes = makeBasicStyle(theme)
  const router = useRouter()
  const { client } = useContext(ValidatorClientContext)

  const [isLoading, setIsLoading] = React.useState<boolean>()
  const [unbondingError, setUnbondingError] = React.useState(null)

  const [checkedOwnership, setCheckedOwnership] = React.useState(false)
  const [ownsMixnode, setOwnsMixnode] = React.useState(false)
  const [ownsGateway, setOwnsGateway] = React.useState(false)

  const [nodeType, setNodeType] = React.useState(NodeType.Mixnode)

  useEffect(() => {
    const checkOwnership = async () => {
      if (client === null) {
        await router.push('/')
      } else {
        const nodeOwnership = await checkNodesOwnership(client).finally(() =>
          setCheckedOwnership(true)
        )
        setOwnsMixnode(nodeOwnership.ownsMixnode)
        setOwnsGateway(nodeOwnership.ownsGateway)
        if (nodeOwnership.ownsGateway) {
          setNodeType(NodeType.Gateway)
        }
      }
    }
    checkOwnership()
  }, [client])

  const unbondNode = async (event) => {
    setIsLoading(true)
    event.preventDefault()
    console.log(`UNBOND button pressed`)

    if (nodeType == NodeType.Mixnode) {
      client
        .unbondMixnode()
        .then((value) => console.log('unbonded mixnode!', value))
        .catch((err) => setUnbondingError(err))
        .finally(() => setIsLoading(false))
    } else {
      client
        .unbondGateway()
        .then((value) => console.log('unbonded gateway!', value))
        .catch((err) => setUnbondingError(err))
        .finally(() => setIsLoading(false))
    }
  }

  const getUnbondContent = () => {
    // we're not signed in
    if (client === null) {
      return <NoClientError />
    }

    // we haven't checked whether we actually own a node to unbond
    if (!checkedOwnership) {
      return <LinearProgress />
    }

    // somehow this address has both a mixnode and a gateway bonded - this is super undesirable
    // if that happens it means the user must have sent transactions outside the wallet before the contract update
    // so they can send transactions outside the wallet to fix themselves up
    if (ownsMixnode && ownsGateway) {
      return (
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <Typography gutterBottom>
              You seem to have both a mixnode and a gateway bonded - how the
              hell did you manage to do that?
            </Typography>
          </Grid>
        </Grid>
      )
    }

    // we don't own anything
    if (!ownsMixnode && !ownsGateway) {
      return (
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <Typography gutterBottom>
              You do not currently have a mixnode or a gateway bonded.
            </Typography>
          </Grid>
        </Grid>
      )
    }

    // we haven't clicked unbond button yet
    if (isLoading === undefined) {
      return <UnbondNotice onClick={unbondNode} />
    }

    // We started unbonding
    return (
      <Confirmation
        isLoading={isLoading}
        error={unbondingError}
        progressMessage={`${nodeType} unbonding is in progress...`}
        successMessage={`${nodeType} unbonding was successful!`}
        failureMessage={`Failed to unbond the ${nodeType}!`}
      />
    )
  }

  let headerText = 'Node'
  if (ownsGateway || ownsGateway) {
    headerText = nodeType
  }

  return (
    <>
      <main className={classes.layout}>
        <Paper className={classes.paper}>
          <ExecFeeNotice name={'unbonding'} />
          <Typography
            component="h1"
            variant="h4"
            align="center"
            className={classes.wrapper}
          >
            Unbond a {headerText}
          </Typography>
          {getUnbondContent()}
        </Paper>
      </main>
    </>
  )
}

export default UnbondNode
