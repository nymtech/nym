import React from 'react';
import { Box, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, TableSortLabel } from '@mui/material';
import { visuallyHidden } from '@mui/utils';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import { DelegationWithEverything } from '@nymproject/types';
import { useSortDelegations } from 'src/hooks/useSortDelegations';
import { DelegationListItemActions } from './DelegationActions';
import { DelegationItem } from './DelegationItem';
import { PendingDelegationItem } from './PendingDelegationItem';
import { LoadingModal } from '../Modals/LoadingModal';
import { isDelegation, isPendingDelegation, TDelegations, useDelegationContext } from '../../context/delegations';
import { ErrorModal } from '../Modals/ErrorModal';

export type Order = 'asc' | 'desc';
type AdditionalTypes = { profit_margin_percent: number; operating_cost: number };
export type SortingKeys = keyof AdditionalTypes | keyof DelegationWithEverything;

interface EnhancedTableProps {
  onRequestSort: (event: React.MouseEvent<unknown>, property: string) => void;
  order: Order;
  orderBy: string;
}

interface HeadCell {
  id: string;
  label: string | React.ReactNode;
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
  {
    id: 'delegated_on_iso_datetime',
    label: 'Delegated on',
    sortable: true,
    align: 'left',
  },
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

export const DelegationList: FCWithChildren<{
  isLoading?: boolean;
  items: TDelegations;
  onItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
  explorerUrl: string;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
}> = ({ isLoading, items, onItemActionClick, explorerUrl }) => {
  const [order, setOrder] = React.useState<Order>('asc');
  const [orderBy, setOrderBy] = React.useState<SortingKeys>('delegated_on_iso_datetime');

  const { delegationItemErrors, setDelegationItemErrors } = useDelegationContext();

  const handleRequestSort = (_: React.MouseEvent<unknown>, property: any) => {
    const isAsc = orderBy === property && order === 'asc';
    setOrder(isAsc ? 'desc' : 'asc');
    setOrderBy(property);
  };

  const sorted = useSortDelegations(items, order, orderBy);

  return (
    <TableContainer>
      {isLoading && <LoadingModal text="Please wait. Refreshing..." />}
      <ErrorModal
        open={Boolean(delegationItemErrors)}
        title={`Delegation errors for Node ID ${delegationItemErrors?.nodeId || 'unknown'}`}
        message={delegationItemErrors?.errors || 'oops'}
        onClose={() => setDelegationItemErrors(undefined)}
      />
      <Table sx={{ width: '100%' }}>
        <EnhancedTableHead order={order} orderBy={orderBy} onRequestSort={handleRequestSort} />
        <TableBody>
          {sorted?.length
            ? sorted.map((item: any) => {
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
            : null}
        </TableBody>
      </Table>
    </TableContainer>
  );
};
