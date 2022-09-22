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
import { visuallyHidden } from '@mui/utils';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import { decimalToPercentage, DelegationWithEverything } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { format } from 'date-fns';
import { DelegationListItemActions, DelegationsActionsMenu } from './DelegationActions';
import { DelegationWithEvent, isDelegation, isPendingDelegation, TDelegations } from '../../context/delegations';
import { toPercentIntegerString } from '../../utils';

type Order = 'asc' | 'desc';

interface EnhancedTableProps {
  onRequestSort: (event: React.MouseEvent<unknown>, property: string) => void;
  order: Order;
  orderBy: string;
}

interface HeadCell {
  id: string;
  label: string;
  sortable: boolean;
  disablePadding?: boolean;
  align: 'left' | 'center' | 'right';
}

const headCells: HeadCell[] = [
  { id: 'node_identity', label: 'Node ID', sortable: true, align: 'left' },
  { id: 'avg_uptime_percent', label: 'Uptime', sortable: true, align: 'left' },
  { id: 'profit_margin_percent', label: 'Profit margin', sortable: true, align: 'left' },
  { id: 'stake_saturation', label: 'Stake saturation', sortable: true, align: 'left' },
  { id: 'delegated_on_iso_datetime', label: 'Delegated on', sortable: true, align: 'left' },
  { id: 'amount', label: 'Delegation', sortable: true, align: 'left' },
  { id: 'unclaimed_rewards', label: 'Reward', sortable: true, align: 'left' },
];

function descendingComparator(a: any, b: any, orderBy: string) {
  if (b[orderBy] < a[orderBy]) {
    return -1;
  }
  if (b[orderBy] > a[orderBy]) {
    return 1;
  }
  return 0;
}

function sortPendingDelegation(a: DelegationWithEvent, b: DelegationWithEvent) {
  if (isPendingDelegation(a) && isPendingDelegation(b)) return 0;
  if (isPendingDelegation(b)) return -1;
  if (isPendingDelegation(a)) return 1;
  return 2;
}

function getComparator(order: Order, orderBy: string): (a: DelegationWithEvent, b: DelegationWithEvent) => number {
  return order === 'desc'
    ? (a, b) => {
        const pendingSort = sortPendingDelegation(a, b);
        if (pendingSort === 2) return descendingComparator(a, b, orderBy);
        return pendingSort;
      }
    : (a, b) => {
        const pendingSort = -sortPendingDelegation(a, b);
        if (pendingSort === 2) return -descendingComparator(a, b, orderBy);
        return pendingSort;
      };
}

const EnhancedTableHead: React.FC<EnhancedTableProps> = ({ order, orderBy, onRequestSort }) => {
  const createSortHandler = (property: string) => (event: React.MouseEvent<unknown>) => {
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
  items?: TDelegations;
  onItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
  explorerUrl: string;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
}> = ({ isLoading, items, onItemActionClick, explorerUrl }) => {
  const [order, setOrder] = React.useState<Order>('asc');
  const [orderBy, setOrderBy] = React.useState<string>('delegated_on_iso_datetime');

  const handleRequestSort = (event: React.MouseEvent<unknown>, property: string) => {
    const isAsc = orderBy === property && order === 'asc';
    setOrder(isAsc ? 'desc' : 'asc');
    setOrderBy(property);
  };

  const getStakeSaturation = (item: DelegationWithEvent) => {
    if (isDelegation(item)) {
      return !item.stake_saturation ? '-' : `${decimalToPercentage(item.stake_saturation)}%`;
    }
    return '';
  };

  const getRewardValue = (item: DelegationWithEvent) => {
    if (isPendingDelegation(item)) {
      return '';
    }
    // eslint-disable-next-line @typescript-eslint/naming-convention
    const { unclaimed_rewards } = item;
    return !unclaimed_rewards ? '-' : `${unclaimed_rewards.amount} ${unclaimed_rewards.denom}`;
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
                  <Link
                    target="_blank"
                    href={`${explorerUrl}/network-components/mixnode/${item.node_identity}`}
                    text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
                    color="text.primary"
                    noIcon
                  />
                </TableCell>
                <TableCell>
                  {isDelegation(item) && (!item.avg_uptime_percent ? '-' : `${item.avg_uptime_percent}%`)}
                </TableCell>
                <TableCell>
                  {isDelegation(item) &&
                    (!item.cost_params?.profit_margin_percent
                      ? '-'
                      : `${toPercentIntegerString(item.cost_params.profit_margin_percent)}%`)}
                </TableCell>
                <TableCell>{getStakeSaturation(item)}</TableCell>
                <TableCell>
                  {isDelegation(item) && format(new Date(item.delegated_on_iso_datetime), 'dd/MM/yyyy')}
                </TableCell>
                <TableCell>
                  <Typography style={{ textTransform: 'uppercase' }}>
                    {isDelegation(item) && `${item.amount.amount} ${item.amount.denom}`}
                  </Typography>
                </TableCell>
                <TableCell sx={{ textTransform: 'uppercase' }}>{getRewardValue(item)}</TableCell>
                <TableCell align="right">
                  {isDelegation(item) && !item.pending_events.length && (
                    <DelegationsActionsMenu
                      isPending={undefined}
                      onActionClick={(action) => (onItemActionClick ? onItemActionClick(item, action) : undefined)}
                      disableRedeemingRewards={!item.unclaimed_rewards || item.unclaimed_rewards.amount === '0'}
                    />
                  )}
                  {isDelegation(item) && item.pending_events.length > 0 && (
                    <Tooltip
                      title="Your changes will take effect when
the new epoch starts. There is a new
epoch every hour."
                      arrow
                    >
                      <Chip label="Pending Events" />
                    </Tooltip>
                  )}
                  {isPendingDelegation(item) && (
                    <Tooltip
                      title={`Your delegation of ${item.event.amount?.amount} ${item.event.amount?.denom} will take effect 
                        when the new epoch starts. There is a new
                        epoch every hour.`}
                      arrow
                    >
                      <Chip label="Pending Events" />
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
