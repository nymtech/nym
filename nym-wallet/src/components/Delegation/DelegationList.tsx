import React from 'react';
import {
  Box,
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
import { orderBy as _orderBy } from 'lodash';
import { DelegationWithEverything } from '@nymproject/types';
import { DelegationListItemActions } from './DelegationActions';
import { isDelegation, isPendingDelegation, TDelegations } from '../../context/delegations';
import { DelegationItem } from './DelegationItem';
import { PendingDelegationItem } from './PendingDelegationItem';

type Order = 'asc' | 'desc';
type AdditionalTypes = { profit_margin_percent: number; operating_cost: number };
type SortingKeys = keyof AdditionalTypes | keyof DelegationWithEverything;

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
  { id: 'operating_cost', label: 'Operating Cost', sortable: true, align: 'left' },
  { id: 'stake_saturation', label: 'Stake saturation', sortable: true, align: 'left' },
  { id: 'delegated_on_iso_datetime', label: 'Delegated on', sortable: true, align: 'left' },
  { id: 'amount', label: 'Delegation', sortable: true, align: 'left' },
  { id: 'unclaimed_rewards', label: 'Reward', sortable: true, align: 'left' },
  { id: 'uses_locked_tokens', label: '', sortable: false, align: 'left' },
];

const EnhancedTableHead: FCWithChildren<EnhancedTableProps> = ({ order, orderBy, onRequestSort }) => {
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

// Pin delegations on unbonded nodes to the top of the list
const sortByUnbondedMixnodeFirst = (a: any) => {
  if (!a.node_identity) return -1;
  return 1;
};

export const DelegationList: FCWithChildren<{
  isLoading?: boolean;
  items: TDelegations;
  onItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
  explorerUrl: string;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
}> = ({ isLoading, items, onItemActionClick, explorerUrl }) => {
  const [order, setOrder] = React.useState<Order>('asc');
  const [orderBy, setOrderBy] = React.useState<SortingKeys>('delegated_on_iso_datetime');

  const handleRequestSort = (event: React.MouseEvent<unknown>, property: any) => {
    const isAsc = orderBy === property && order === 'asc';
    setOrder(isAsc ? 'desc' : 'asc');
    setOrderBy(property);
  };

  // if sorting by either amount or unclaimed_rewards
  // base sorting on their number counterparts
  const mapOrderBy = (key: SortingKeys) => {
    if (key === 'amount') return 'delegationValue';
    if (key === 'unclaimed_rewards') return 'operatorReward';
    if (key === 'profit_margin_percent') return 'profitMarginValue';
    if (key === 'operating_cost') return 'operatorCostValue';
    return key;
  };

  const mapAndSort = (_items: TDelegations) => {
    const map = _items.map((item) =>
      isDelegation(item)
        ? {
            ...item,
            delegationValue: Number(item.amount.amount),
            operatorReward: Number(item.unclaimed_rewards?.amount),
            profitMarginValue: Number(item.cost_params?.profit_margin_percent),
            operatorCostValue: Number(item.cost_params?.interval_operating_cost),
          }
        : item,
    );

    return _orderBy(map, mapOrderBy(orderBy), order).sort(sortByUnbondedMixnodeFirst);
  };

  return (
    <TableContainer>
      <Table sx={{ width: '100%' }}>
        <EnhancedTableHead order={order} orderBy={orderBy} onRequestSort={handleRequestSort} />
        <TableBody>
          {items.length ? (
            mapAndSort(items).map((item: any) => {
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
