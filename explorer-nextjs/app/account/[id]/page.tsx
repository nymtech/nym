'use client'

import * as React from 'react'
import {Alert, AlertTitle, Box, Button, Chip, CircularProgress, Grid, Tooltip, Typography} from '@mui/material'
import { useParams } from 'next/navigation'
import { useMainContext } from '@/app/context/main'
import { Title } from '@/app/components/Title'
import {MaterialReactTable, MRT_ColumnDef, useMaterialReactTable} from "material-react-table";
import {useMemo} from "react";
import {humanReadableCurrencyToString} from "@/app/utils/currency";
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';
import WarningAmberIcon from '@mui/icons-material/WarningAmber';
import { PieChart } from '@mui/x-charts/PieChart';
import {useTheme} from "@mui/material/styles";
import {useIsMobile} from "@/app/hooks";

const AccumulatedRewards = ({account}: { account?: any}) => {
  const columns = useMemo<
    MRT_ColumnDef<any>[]
  >(() => {
    return [
      {
        id: 'accumulated-rewards-data',
        header: 'Accumulated Rewards Data',
        columns: [
          {
            id: 'node_id',
            accessorKey: 'node_id',
            header: 'Node ID',
            size: 150,
          },
          {
            id: 'node_still_fully_bonded',
            accessorKey: 'node_still_fully_bonded',
            header: 'Node still bonded?',
            width: 150,
            Cell: ({ row }) => (
              <>{row.original.node_still_fully_bonded ? <CheckCircleOutlineIcon/> :
                <Typography fontSize="inherit" alignItems="center" display="flex" sx={{ color: theme => theme.palette.warning.main }}>
                  <WarningAmberIcon sx={{ mr: 1 }}/>
                  Unbonded
                </Typography>}</>
            )
          },
          {
            id: 'amount_staked',
            accessorKey: 'amount_staked',
            header: 'Amount',
            width: 150,
            Cell: ({ row }) => (
              <>{humanReadableCurrencyToString(row.original.amount_staked)}</>
            )
          },
          {
            id: 'rewards',
            accessorKey: 'rewards',
            header: 'Rewards',
            width: 150,
            Cell: ({ row }) => (
              <Typography fontSize="inherit" color="success.main">{humanReadableCurrencyToString(row.original.rewards)}</Typography>
            )
          },
        ],
      },
    ]
  }, [])

  const table = useMaterialReactTable({
    columns,
    data: account?.accumulated_rewards || [],
    enableFullScreenToggle: false,
  })

  return (<MaterialReactTable table={table} />);
}

const DelegationHistory = ({account}: { account?: any}) => {
  const columns = useMemo<
    MRT_ColumnDef<any>[]
  >(() => {
    return [
      {
        id: 'delegation-history-data',
        header: 'Delegation History',
        columns: [
          {
            id: 'node_id',
            accessorKey: 'node_id',
            header: 'Node ID',
            size: 150,
          },
          {
            id: 'delegated',
            accessorKey: 'delegated',
            header: 'Amount',
            width: 150,
            Cell: ({ row }) => (
              <>{humanReadableCurrencyToString(row.original.delegated)}</>
            )
          },
          {
            id: 'height',
            accessorKey: 'height',
            header: 'Delegated at height',
            width: 150,
            Cell: ({ row }) => (
              <>{row.original.height}</>
            )
          },
        ],
      },
    ]
  }, [])

  const table = useMaterialReactTable({
    columns,
    data: account?.delegations || [],
    enableFullScreenToggle: false,
  })

  return (<MaterialReactTable table={table} />);
}


/**
 * Shows account details
 */
const PageAccountWithState = ({ account }: {
  account?: any;
}) => {
  const theme = useTheme();
  const isMobile = useIsMobile();

  const pieChartData = React.useMemo(() => {
    if(!account) {
      return [];
    }

    const parts = [];

    const nymBalance = Number.parseFloat(account.balances.find((b: any) => b.denom === "unym")?.amount || "0") / 1e6;

    parts.push({ label: "Spendable", value: nymBalance, color: theme.palette.primary.main });

    if(account.vesting_account) {
      if (`${account.vesting_account.locked?.amount}` !== "0") {
        parts.push({
          label: "Vesting locked",
          value: Number.parseFloat(account.vesting_account.locked.amount) / 1e6,
          color: 'red'
        });
      }
      if (`${account.vesting_account.spendable?.amount}` !== "0") {
        parts.push({
          label: "Vesting spendable",
          value: Number.parseFloat(account.vesting_account.spendable.amount) / 1e6,
          color: theme.palette.primary.light
        });
      }
    }

    if (`${account.claimable_rewards.amount}` !== "0") {
      parts.push({
        label: "Claimable delegation rewards",
        value: Number.parseFloat(account.claimable_rewards.amount) / 1e6,
        color: theme.palette.success.light
      });
    }
    if (`${account.operator_rewards.amount}` !== "0") {
      parts.push({
        label: "Claimable operator rewards",
        value: Number.parseFloat(account.operator_rewards.amount) / 1e6,
        color: theme.palette.success.dark
      });
    }
    if (`${account.total_delegations.amount}` !== "0") {
      parts.push({
        label: "Total delegations",
        value: Number.parseFloat(account.total_delegations.amount) / 1e6,
        color: '#888'
      });
    }

    return parts;
  }, [account]);

  return (
    <Box component="main">
      <Box overflow="scroll">
        <Title text={`Account ${account.address}`} />
      </Box>

      <Box mt={4} sx={{ maxWidth: "600px" }}>
        <PieChart
          series={[
            {
              data: pieChartData,
              innerRadius: 40,
              outerRadius: 80,
              cy: isMobile ? 200 : undefined,
            },
          ]}
          height={300}
          slotProps={isMobile ? {
            legend: { position: { vertical: "top", horizontal: "right" } }
          } : undefined}
        />
      </Box>

      <Box mt={4}>
        <TableContainer component={Paper} sx={{ maxWidth: "400px" }}>
          <Table>
            <TableBody>
              <TableRow sx={{ color: theme => theme.palette.primary.main }}>
                <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                  <strong>Spendable Balance</strong>
                </TableCell>
                <TableCell align="right" sx={{ color: "inherit" }}>
                  {account.balances.map((b: any) => (<strong key={`balance-${b.denom}`}>{humanReadableCurrencyToString(b)}<br/></strong>))}
                </TableCell>
              </TableRow>
              <TableRow>
                <TableCell component="th" scope="row">
                  Total delegations
                </TableCell>
                <TableCell align="right">
                  {humanReadableCurrencyToString(account.total_delegations)}
                </TableCell>
              </TableRow>
              <TableRow sx={{ color: theme => theme.palette.success.light }}>
                <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                  Claimable delegation rewards
                </TableCell>
                <TableCell align="right" sx={{ color: "inherit" }}>
                  {humanReadableCurrencyToString(account.claimable_rewards)}
                </TableCell>
              </TableRow>
              {`${account.operator_rewards.amount}` !== "0" && <TableRow sx={{ color: theme => theme.palette.success.light }}>
                <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                  Claimable operator rewards
                </TableCell>
                <TableCell align="right" sx={{ color: "inherit" }}>
                  {humanReadableCurrencyToString(account.operator_rewards)}
                </TableCell>
              </TableRow>}
              {account.vesting_account && (
                <>
                  <TableRow>
                    <TableCell component="th" scope="row" colSpan={2}>
                      Vesting account
                    </TableCell>
                  </TableRow>
                  {`${account.vesting_account.locked.amount}` !== "0" &&
                    <TableRow>
                        <TableCell component="th" scope="row" sx={{ pl: 4 }}>
                            Locked
                        </TableCell>
                        <TableCell align="right" sx={{ color: "inherit" }}>
                          {humanReadableCurrencyToString(account.vesting_account.locked)}
                        </TableCell>
                    </TableRow>
                  }
                  {`${account.vesting_account.vested.amount}` !== "0" &&
                      <TableRow>
                          <TableCell component="th" scope="row" sx={{ pl: 4 }}>
                              Vested
                          </TableCell>
                          <TableCell align="right" sx={{ color: "inherit" }}>
                            {humanReadableCurrencyToString(account.vesting_account.vested)}
                          </TableCell>
                      </TableRow>
                  }
                  {`${account.vesting_account.vesting.amount}` !== "0" &&
                      <TableRow>
                          <TableCell component="th" scope="row" sx={{ pl: 4 }}>
                              Vesting
                          </TableCell>
                          <TableCell align="right" sx={{ color: "inherit" }}>
                            {humanReadableCurrencyToString(account.vesting_account.vesting)}
                          </TableCell>
                      </TableRow>
                  }
                  {`${account.vesting_account.spendable.amount}` !== "0" &&
                      <TableRow>
                          <TableCell component="th" scope="row" sx={{ pl: 4 }}>
                              Spendable
                          </TableCell>
                          <TableCell align="right" sx={{ color: "inherit" }}>
                            {humanReadableCurrencyToString(account.vesting_account.spendable)}
                          </TableCell>
                      </TableRow>
                  }
                </>
              )}
              <TableRow>
                <TableCell component="th" scope="row" sx={{ color: "inherit" }}>
                  <h3>Total value</h3>
                </TableCell>
                <TableCell align="right" sx={{ color: "inherit" }}>
                  <h3>{humanReadableCurrencyToString(account.total_value)}</h3>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </TableContainer>
      </Box>
      <Box mt={4}>
        <AccumulatedRewards account={account}/>
      </Box>
      <Box mt={4}>
        <DelegationHistory account={account}/>
      </Box>
    </Box>
  )
}

/**
 * Guard component to handle loading and not found states
 */
const PageAccountDetailGuard = ({ account } : { account: string }) => {
  const [accountDetails, setAccountDetails] = React.useState<any>();
  const [isLoading, setLoading] = React.useState<boolean>(true);
  const [error, setError] = React.useState<string>();
  const { fetchAccountById } = useMainContext()
  const { id } = useParams()

  React.useEffect(() => {
    setLoading(true);
    (async () => {
      if(typeof(id) === "string") {
        try {
          const res = await fetchAccountById(account);
          setAccountDetails(res);
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
        <AlertTitle>Account not found</AlertTitle>
        Sorry, we could not find the account <code>{id || ''}</code>
      </Alert>
    )
  }

  return <PageAccountWithState account={accountDetails} />
}

/**
 * Wrapper component that adds the account details based on the `id` in the address URL
 */
const PageAccountDetail = () => {
  const { id } = useParams()

  if (!id || typeof id !== 'string') {
    return (
      <Alert severity="error">Oh no! Could not find that account</Alert>
    )
  }

  return (
    <PageAccountDetailGuard account={id} />
  )
}

export default PageAccountDetail
