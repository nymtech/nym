import React, { useState } from 'react';
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
} from '@mui/material';
import { visuallyHidden } from '@mui/utils';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import { DelegationWithEverything } from '@nymproject/types';
import { DelegationListItemActions } from './DelegationActions';
import { DelegationWithEvent, isDelegation, isPendingDelegation, TDelegations } from '../../context/delegations';
import { DelegationItem } from './DelegationItem';
import { PendingDelegationItem } from './PendingDelegationItem';

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
  { id: 'avg_uptime_percent', label: 'Routing score', sortable: true, align: 'left' },
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
// Sorting function needs fixing

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
  items: TDelegations;
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

  return (
    <TableContainer>
      <Table sx={{ width: '100%' }}>
        <EnhancedTableHead order={order} orderBy={orderBy} onRequestSort={handleRequestSort} />
        <TableBody>
          {items.length ? (
            items.map((item) => {
              if (isPendingDelegation(item)) return <PendingDelegationItem item={item} explorerUrl={explorerUrl} />;
              if (isDelegation(item))
                return (
                  <DelegationItem
                    item={item}
                    explorerUrl={explorerUrl}
                    nodeIsUnbonded={Boolean(!item.node_identity)}
                    onItemActionClick={onItemActionClick}
                  />
                );
              return null;
            })
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
