import React from 'react';
import {
  Alert,
  AlertTitle,
  Box,
  Button,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TableSortLabel,
  Typography,
} from '@mui/material';
import { visuallyHidden } from '@mui/utils';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import { DelegationWithEverything } from '@nymproject/types';
import { useSortDelegations } from 'src/hooks/useSortDelegations';
import { useNavigate } from 'react-router-dom';
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

const hasPruningError = (item: any): boolean => {
  if (!isDelegation(item) || !item.errors) return false;

  return (
    (item.errors.includes('height') && item.errors.includes('not available')) ||
    item.errors.includes('Due to pruning strategies')
  );
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
  const navigate = useNavigate();

  const { delegationItemErrors, setDelegationItemErrors } = useDelegationContext();

  const handleRequestSort = (_: React.MouseEvent<unknown>, property: any) => {
    const isAsc = orderBy === property && order === 'asc';
    setOrder(isAsc ? 'desc' : 'asc');
    setOrderBy(property);
  };

  const sorted = useSortDelegations(items, order, orderBy);

  // Check if any delegations have pruning errors
  const hasPruningErrors = React.useMemo(() => sorted?.some((item) => hasPruningError(item)), [sorted]);

  // Navigate to settings page
  const navigateToSettings = () => {
    navigate('/settings');
  };

  // Format error message for display
  const formatErrorMessage = (message: string) => {
    if (message.includes('height') && message.includes('not available')) {
      return 'Due to pruning strategies from validators, please navigate to the Settings tab and change your RPC node for your validator to retrieve your delegations.';
    }
    return message;
  };

  return (
    <>
      {/* Display pruning error alert at the top if needed */}
      {hasPruningErrors && (
        <Alert
          severity="warning"
          sx={{ mb: 2 }}
          action={
            <Button color="inherit" size="small" onClick={navigateToSettings}>
              Go to Settings
            </Button>
          }
        >
          <AlertTitle>Data Pruning Detected</AlertTitle>
          <Typography>
            Some delegation details cannot be retrieved because of data pruning on the validator. Please navigate to the
            Settings tab and change your RPC node to fix this issue.
          </Typography>
        </Alert>
      )}

      <TableContainer>
        {isLoading && <LoadingModal text="Please wait. Refreshing..." />}
        <ErrorModal
          open={Boolean(delegationItemErrors)}
          title={`Delegation errors for Node ID ${delegationItemErrors?.nodeId || 'unknown'}`}
          message={
            delegationItemErrors?.errors ? formatErrorMessage(delegationItemErrors.errors) : 'An unknown error occurred'
          }
          onClose={() => setDelegationItemErrors(undefined)}
        />
        <Table sx={{ width: '100%' }}>
          <EnhancedTableHead order={order} orderBy={orderBy} onRequestSort={handleRequestSort} />
          <TableBody>
            {sorted?.length
              ? sorted.map((item: any, _index: number) => {
                  if (isPendingDelegation(item)) {
                    const pendingKey = `pending-${item.event.mix_id}-${item.event.address}-${item.node_identity}`;
                    return <PendingDelegationItem key={pendingKey} item={item} explorerUrl={explorerUrl} />;
                  }
                  if (isDelegation(item))
                    return (
                      <DelegationItem
                        key={`delegation-${item.mix_id}`}
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
    </>
  );
};
