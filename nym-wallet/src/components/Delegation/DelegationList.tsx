import React from 'react';
import {
  Box,
  Chip,
  CircularProgress,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TableSortLabel,
  Tooltip,
  Typography,
} from '@mui/material';
import LockOutlinedIcon from '@mui/icons-material/LockOutlined';
import { visuallyHidden } from '@mui/utils';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard';
import { DelegationWithEverything } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { format, formatDistanceToNow, parseISO } from 'date-fns';
import { styled } from '@mui/material/styles';
import { tableCellClasses } from '@mui/material/TableCell';
import { DelegationListItemActions, DelegationsActionsMenu } from './DelegationActions';

const StyledTooltipTableCell = styled(TableCell)(({ theme }) => ({
  [`&.${tableCellClasses.head}`]: {
    color: theme.palette.common.white,
    opacity: 0.5,
    fontSize: 12,
  },
  [`&.${tableCellClasses.body}`]: {
    color: theme.palette.common.white,
    fontSize: 12,
  },
}));

type Order = 'asc' | 'desc';

interface EnhancedTableProps {
  onRequestSort: (event: React.MouseEvent<unknown>, property: keyof DelegationWithEverything) => void;
  order: Order;
  orderBy: string;
}

interface HeadCell {
  id: keyof DelegationWithEverything;
  label: string;
  sortable: boolean;
  disablePadding?: boolean;
  align: 'left' | 'center' | 'right';
}

const headCells: HeadCell[] = [
  { id: 'node_identity', label: 'Node ID', sortable: true, align: 'left' },
  { id: 'avg_uptime_percent', label: 'Uptime', sortable: true, align: 'center' },
  { id: 'profit_margin_percent', label: 'Profit margin', sortable: true, align: 'center' },
  { id: 'stake_saturation', label: 'Stake saturation', sortable: true, align: 'center' },
  { id: 'delegated_on_iso_datetime', label: 'Delegated on', sortable: true, align: 'center' },
  { id: 'amount', label: 'Delegation', sortable: true, align: 'center' },
  { id: 'accumulated_rewards', label: 'Reward', sortable: true, align: 'center' },
];

function descendingComparator<T>(a: T, b: T, orderBy: keyof T) {
  if (b[orderBy] < a[orderBy]) {
    return -1;
  }
  if (b[orderBy] > a[orderBy]) {
    return 1;
  }
  return 0;
}

function getComparator<Key extends keyof DelegationWithEverything>(
  order: Order,
  orderBy: Key,
): (a: DelegationWithEverything, b: DelegationWithEverything) => number {
  return order === 'desc'
    ? (a, b) => descendingComparator(a, b, orderBy)
    : (a, b) => -descendingComparator(a, b, orderBy);
}

const EnhancedTableHead: React.FC<EnhancedTableProps> = ({ order, orderBy, onRequestSort }) => {
  const createSortHandler = (property: keyof DelegationWithEverything) => (event: React.MouseEvent<unknown>) => {
    onRequestSort(event, property);
  };

  return (
    <TableHead>
      <TableRow>
        {headCells.map((headCell) => (
          <TableCell
            key={headCell.id}
            align={headCell.align}
            padding={headCell.disablePadding ? 'none' : 'normal'}
            sortDirection={orderBy === headCell.id ? order : false}
            color="secondary"
          >
            <TableSortLabel
              active={orderBy === headCell.id}
              direction={orderBy === headCell.id ? order : 'asc'}
              onClick={createSortHandler(headCell.id)}
              IconComponent={ArrowDropDownIcon}
            >
              {headCell.label}
              {orderBy === headCell.id ? (
                <Box component="span" sx={visuallyHidden}>
                  {order === 'desc' ? 'sorted descending' : 'sorted ascending'}
                </Box>
              ) : null}
            </TableSortLabel>
          </TableCell>
        ))}
        <TableCell />
      </TableRow>
    </TableHead>
  );
};

export const DelegationList: React.FC<{
  isLoading?: boolean;
  items?: DelegationWithEverything[];
  onItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
  explorerUrl: string;
}> = ({ isLoading, items, onItemActionClick, explorerUrl }) => {
  const [order, setOrder] = React.useState<Order>('asc');
  const [orderBy, setOrderBy] = React.useState<keyof DelegationWithEverything>('delegated_on_iso_datetime');

  const handleRequestSort = (event: React.MouseEvent<unknown>, property: keyof DelegationWithEverything) => {
    const isAsc = orderBy === property && order === 'asc';
    setOrder(isAsc ? 'desc' : 'asc');
    setOrderBy(property);
  };

  return (
    <TableContainer>
      <Table sx={{ width: '100%' }}>
        <EnhancedTableHead order={order} orderBy={orderBy} onRequestSort={handleRequestSort} />
        <TableBody>
          {items?.length ? (
            items.sort(getComparator(order, orderBy)).map((item) => (
              <TableRow key={item.node_identity}>
                <TableCell>
                  <CopyToClipboard
                    sx={{ fontSize: 16, mr: 1 }}
                    value={item.node_identity}
                    tooltip={
                      <>
                        Copy identity key <strong>{item.node_identity}</strong> to clipboard
                      </>
                    }
                  />
                  <Tooltip
                    title={
                      <>
                        Click to view <strong>{item.node_identity}</strong> in the Network Explorer
                      </>
                    }
                    placement="right"
                    arrow
                  >
                    <Link
                      target="_blank"
                      href={`${explorerUrl}/network-components/mixnode/${item.node_identity}`}
                      text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
                    />
                  </Tooltip>
                </TableCell>
                <TableCell align="center">{!item.avg_uptime_percent ? '-' : `${item.avg_uptime_percent}%`}</TableCell>
                <TableCell align="center">
                  {!item.profit_margin_percent ? '-' : `${item.profit_margin_percent}%`}
                </TableCell>
                <TableCell align="center">
                  {!item.stake_saturation ? '-' : `${Math.round(item.stake_saturation * 100000) / 1000}%`}
                </TableCell>
                <TableCell align="center">{format(new Date(item.delegated_on_iso_datetime), 'dd/MM/yyyy')}</TableCell>
                <TableCell align="center">
                  <Tooltip
                    placement="right"
                    title={
                      <TableContainer component={Box} color="white">
                        <Table size="small">
                          <TableHead>
                            <TableRow>
                              <StyledTooltipTableCell>Date</StyledTooltipTableCell>
                              <StyledTooltipTableCell>Amount</StyledTooltipTableCell>
                              <StyledTooltipTableCell>Block Height</StyledTooltipTableCell>
                            </TableRow>
                          </TableHead>
                          <TableBody>
                            {item.history.map((historyItem) => (
                              <TableRow key={`${historyItem.block_height}`}>
                                <StyledTooltipTableCell>
                                  {formatDistanceToNow(parseISO(historyItem.delegated_on_iso_datetime), {
                                    addSuffix: true,
                                  })}
                                </StyledTooltipTableCell>
                                <StyledTooltipTableCell>
                                  <Typography fontSize="inherit" noWrap>
                                    {`${historyItem.amount.amount} ${historyItem.amount.denom}`}
                                    {historyItem.uses_vesting_contract_tokens && (
                                      <LockOutlinedIcon fontSize="inherit" sx={{ ml: 0.5 }} />
                                    )}
                                  </Typography>
                                </StyledTooltipTableCell>
                                <StyledTooltipTableCell>{historyItem.block_height}</StyledTooltipTableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </TableContainer>
                    }
                    arrow
                  >
                    <span style={{ cursor: 'pointer' }}>{`${item.amount.amount} ${item.amount.denom}`}</span>
                  </Tooltip>
                </TableCell>
                <TableCell align="center">
                  {!item.accumulated_rewards
                    ? '-'
                    : `${item.accumulated_rewards.amount} ${item.accumulated_rewards.denom}`}
                </TableCell>

                <TableCell align="right">
                  {!item.pending_events.length ? (
                    <DelegationsActionsMenu
                      isPending={undefined}
                      onActionClick={(action) => (onItemActionClick ? onItemActionClick(item, action) : undefined)}
                      disableRedeemingRewards={!item.accumulated_rewards || item.accumulated_rewards.amount === '0'}
                      disableCompoundRewards={!item.accumulated_rewards || item.accumulated_rewards.amount === '0'}
                    />
                  ) : (
                    <Tooltip
                      title="There will be a new epoch roughly every hour when your changes will take effect"
                      arrow
                    >
                      <Chip label="Pending events" />
                    </Tooltip>
                  )}
                </TableCell>
              </TableRow>
            ))
          ) : (
            <TableRow>
              <TableCell colSpan={7}>
                <Box py={6} textAlign="center">
                  {isLoading ? <CircularProgress /> : <span>You have not delegated to any mixnodes</span>}
                </Box>
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </TableContainer>
  );
};
