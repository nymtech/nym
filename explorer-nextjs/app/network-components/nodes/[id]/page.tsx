'use client'

import * as React from 'react'
import { Alert, AlertTitle, Box, CircularProgress, Grid } from '@mui/material'
import { useParams } from 'next/navigation'
import { ColumnsType, DetailTable } from '@/app/components/DetailTable'
import { ComponentError } from '@/app/components/ComponentError'
import { ContentCard } from '@/app/components/ContentCard'
import { UptimeChart } from '@/app/components/UptimeChart'
import {
  NymNodeContextProvider,
  useNymNodeContext,
} from '@/app/context/node'
import { useMainContext } from '@/app/context/main'
import { Title } from '@/app/components/Title'
import Paper from "@mui/material/Paper";
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableRow from '@mui/material/TableRow';
import {humanReadableCurrencyToString} from "@/app/utils/currency";
import {DeclaredRole} from "@/app/network-components/nodes/DeclaredRole";
import {NodeDelegationsTable, VestingDelegationWarning} from "@/app/network-components/nodes/[id]/NodeDelegationsTable";

const columns: ColumnsType[] = [
  {
    field: 'identity_key',
    title: 'Identity Key',
    headerAlign: 'left',
    width: 230,
  },
  {
    field: 'bond',
    title: 'Bond',
    headerAlign: 'left',
  },
  {
    field: 'host',
    title: 'IP',
    headerAlign: 'left',
    width: 99,
  },
  {
    field: 'location',
    title: 'Location',
    headerAlign: 'left',
  },
  {
    field: 'owner',
    title: 'Owner',
    headerAlign: 'left',
  },
  {
    field: 'version',
    title: 'Version',
    headerAlign: 'left',
  },
]

interface NodeEnrichedRowType {
  node_id: number;
  identity_key: string;
  bond: string;
  host: string;
  location: string;
  owner: string;
  version: string;
}

function nodeEnrichedToGridRow(node: any): NodeEnrichedRowType {
  return {
    node_id: node.node_id,
    owner: node.bond_information?.owner || '',
    identity_key: node.bond_information?.node?.identity_key || '',
    location: node.location?.country_name || '',
    bond: node.bond_information?.original_pledge.amount || 0, // TODO: format
    host: node.bond_information?.node?.host || '',
    version: node.description?.build_information?.build_version || '',
  };
}


/**
 * Shows nym node details
 */
const PageNymNodeDetailsWithState = ({
  selectedNymNode,
}: {
  selectedNymNode?: any
}) => {
  const { uptimeHistory } = useNymNodeContext()
  const enrichedData = React.useMemo(() => selectedNymNode ? [nodeEnrichedToGridRow(selectedNymNode)] : [], []);

  const hasVestingContractDelegations = React.useMemo(() => selectedNymNode?.delegations?.filter((d: any) => d.proxy)?.length, [selectedNymNode]);

  return (
    <Box component="main">
      <Title text="Nym Node Detail" />

      <Grid container mt={4}>
        <Grid item xs={12}>
          <DetailTable
            columnsData={columns}
            tableName="Node detail table"
            rows={enrichedData}
          />
        </Grid>
      </Grid>

      <Grid container mt={2} spacing={2}>
        {selectedNymNode.rewarding_details &&
            <Grid item xs={12} md={4}>
              <TableContainer component={Paper}>
                <Table>
                  <TableBody>
                    <TableRow>
                        <TableCell colSpan={2}>
                            Delegations and Rewards
                        </TableCell>
                    </TableRow>
                    <TableRow>
                      <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                        <strong>Operator</strong>
                      </TableCell>
                      <TableCell align="right">
                        {humanReadableCurrencyToString({ amount : selectedNymNode.rewarding_details.operator.split('.')[0], denom: "unym" })}
                      </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                            <strong>
                              {hasVestingContractDelegations ?
                                <VestingDelegationWarning plural={true}>
                                  Delegates&nbsp;({selectedNymNode.rewarding_details.unique_delegations} delegates)
                                </VestingDelegationWarning> :
                                <>Delegates&nbsp;({selectedNymNode.rewarding_details.unique_delegations} delegates)</>
                              }
                            </strong>
                        </TableCell>
                        <TableCell align="right">
                          {humanReadableCurrencyToString({ amount : selectedNymNode.rewarding_details.delegates.split('.')[0], denom: "unym" })}
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                            <strong>Profit margin</strong>
                        </TableCell>
                        <TableCell align="right">
                          {selectedNymNode.rewarding_details.cost_params.profit_margin_percent * 100}%
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                            <strong>Operator costs</strong>
                        </TableCell>
                        <TableCell align="right">
                          {humanReadableCurrencyToString(selectedNymNode.rewarding_details.cost_params.interval_operating_cost)}
                        </TableCell>
                    </TableRow>
                  </TableBody>
                </Table>
              </TableContainer>
            </Grid>}

        {selectedNymNode.description?.declared_role && <Grid item xs={12} md={4}>
          <TableContainer component={Paper}>
            <Table>
              <TableBody>
                <TableRow>
                  <TableCell colSpan={2}>
                    Node roles
                  </TableCell>
                </TableRow>
                <TableRow>
                  <TableCell>
                    Self declared roles
                  </TableCell>
                  <TableCell>
                    <DeclaredRole declared_role={selectedNymNode.description?.declared_role}/>
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </TableContainer>
        </Grid>}
      </Grid>

      <Grid container spacing={2} mt={2}>
        <Grid item xs={12} md={8}>
          {uptimeHistory && (
            <ContentCard title="Routing Score">
              {uptimeHistory.error && (
                <ComponentError text="There was a problem retrieving routing score." />
              )}
              <UptimeChart
                loading={uptimeHistory.isLoading}
                xLabel="Date"
                yLabel="Daily average"
                uptimeStory={uptimeHistory}
              />
            </ContentCard>
          )}
        </Grid>
      </Grid>

      <Box mt={2}>
        <NodeDelegationsTable node={selectedNymNode}/>
      </Box>
    </Box>
  )
}

/**
 * Guard component to handle loading and not found states
 */
const PageNymNodeDetailGuard = () => {
  const [selectedNode, setSelectedNode] = React.useState<any>()
  const [isLoading, setLoading] = React.useState<boolean>(true);
  const [error, setError] = React.useState<string>();
  const { fetchNodeById } = useMainContext()
  const { id } = useParams()

  React.useEffect(() => {
    setSelectedNode(undefined);
    setLoading(true);
    (async () => {
      if(typeof(id) === "string") {
        try {
          const res = await fetchNodeById(Number.parseInt(id));
          setSelectedNode(res);
        } catch(e: any) {
          setError(e.message);
        }
        finally {
          setLoading(false);
        }
      }
    })();
  }, [id])

  if (isLoading) {
    return <CircularProgress />
  }

  // loaded, but not found
  if (error) {
    return (
      <Alert severity="warning">
        <AlertTitle>Nym node not found</AlertTitle>
        Sorry, we could not find a node with id <code>{id || ''}</code>
      </Alert>
    )
  }

  return <PageNymNodeDetailsWithState selectedNymNode={selectedNode} />
}

/**
 * Wrapper component that adds the node content based on the `id` in the address URL
 */
const PageNymNodeDetail = () => {
  const { id } = useParams()

  if (!id || typeof id !== 'string') {
    return (
      <Alert severity="error">Oh no! Could not find that node</Alert>
    )
  }

  return (
    <NymNodeContextProvider nymNodeId={id}>
      <PageNymNodeDetailGuard />
    </NymNodeContextProvider>
  )
}

export default PageNymNodeDetail
