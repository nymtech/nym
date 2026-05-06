import React from 'react';
import {
  Alert,
  AlertTitle,
  Box,
  Button,
  Collapse,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Skeleton,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material';
import { alpha } from '@mui/material/styles';
import { KeyboardArrowDown, KeyboardArrowUp, LockOutlined, WarningAmberOutlined } from '@mui/icons-material';
import { decimalToFloatApproximation, decimalToPercentage, DelegationWithEverything } from '@nymproject/types';
import { useSortDelegations } from 'src/hooks/useSortDelegations';
import { useNavigate } from 'react-router-dom';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';
import { format } from 'date-fns';
import { Undelegate } from 'src/svg-icons';
import { toPercentIntegerString } from 'src/utils';
import { InfoTooltip } from '../InfoToolTip';
import { DelegationListItemActions, DelegationsActionsMenu } from './DelegationActions';
import { PendingDelegationCard } from './PendingDelegationCard';
import { isDelegation, isPendingDelegation, TDelegations, useDelegationContext } from '../../context/delegations';
import { ErrorModal } from '../Modals/ErrorModal';

export type Order = 'asc' | 'desc';
type AdditionalTypes = { profit_margin_percent: number; operating_cost: number };
export type SortingKeys = keyof AdditionalTypes | keyof DelegationWithEverything;

const shouldBeFiltered = (item: any): boolean => {
  if (isDelegation(item)) {
    if (!item.node_identity || item.node_identity === '-' || item.node_identity === '...') {
      return true;
    }
    if (typeof item.avg_uptime_percent === 'string' && item.avg_uptime_percent === '-') {
      return true;
    }
  }

  if (isPendingDelegation(item)) {
    if ((!item.node_identity || item.node_identity === '') && item.event && item.event.kind === 'Undelegate') {
      return true;
    }
    return false;
  }

  return false;
};

const SORT_FIELD_OPTIONS: { id: SortingKeys; label: string }[] = [
  { id: 'delegated_on_iso_datetime', label: 'Delegated on' },
  { id: 'node_identity', label: 'Node ID' },
  { id: 'avg_uptime_percent', label: 'Routing score' },
  { id: 'profit_margin_percent', label: 'Profit margin' },
  { id: 'operating_cost', label: 'Operating cost' },
  { id: 'stake_saturation', label: 'Stake saturation' },
  { id: 'amount', label: 'Delegation' },
  { id: 'unclaimed_rewards', label: 'Reward' },
];

const hasPruningError = (item: any): boolean => {
  if (!isDelegation(item) || !item.errors) return false;

  return (
    (item.errors.includes('height') && item.errors.includes('not available')) ||
    item.errors.includes('Due to pruning strategies')
  );
};

const getStakeSaturation = (item: DelegationWithEverything) =>
  !item.stake_saturation ? '-' : `${decimalToPercentage(item.stake_saturation)}%`;

const getRewardValue = (item: DelegationWithEverything) => {
  const { unclaimed_rewards } = item;
  return !unclaimed_rewards ? '-' : `${unclaimed_rewards.amount} ${unclaimed_rewards.denom}`;
};

const saturationNumeric = (item: DelegationWithEverything): number | undefined => {
  if (!item.stake_saturation) return undefined;
  return decimalToFloatApproximation(item.stake_saturation);
};

export const DelegationList: FCWithChildren<{
  items: TDelegations;
  onItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
  explorerUrl: string;
  nextEpoch?: string | Error;
}> = ({ items, onItemActionClick, explorerUrl, nextEpoch }) => {
  const [order, setOrder] = React.useState<Order>('asc');
  const [orderBy, setOrderBy] = React.useState<SortingKeys>('delegated_on_iso_datetime');
  const [identityFilter, setIdentityFilter] = React.useState('');
  const [expandedKey, setExpandedKey] = React.useState<string | null>(null);
  const navigate = useNavigate();

  const {
    delegationItemErrors,
    setDelegationItemErrors,
    totalDelegations,
    totalRewards,
    totalDelegationsAndRewards,
    isFetching: delegationsSummaryLoading,
  } = useDelegationContext();

  const sorted = useSortDelegations(items, order, orderBy);

  const filteredItems = React.useMemo(() => {
    if (!sorted) return [];
    return sorted.filter((item) => !shouldBeFiltered(item));
  }, [sorted]);

  const activeDelegations = React.useMemo(
    () => filteredItems.filter((item): item is DelegationWithEverything => isDelegation(item)),
    [filteredItems],
  );

  const pendingItems = React.useMemo(() => filteredItems.filter((item) => isPendingDelegation(item)), [filteredItems]);

  const searchNeedle = identityFilter.trim().toLowerCase();

  const displayedDelegations = React.useMemo(() => {
    if (!searchNeedle) return activeDelegations;
    return activeDelegations.filter((d) => d.node_identity.toLowerCase().includes(searchNeedle));
  }, [activeDelegations, searchNeedle]);

  const activeCount = activeDelegations.length;

  const hasPruningErrors = React.useMemo(() => filteredItems?.some((item) => hasPruningError(item)), [filteredItems]);

  const navigateToSettings = () => {
    navigate('/settings');
  };

  const formatErrorMessage = (message: string) => {
    if (message.includes('height') && message.includes('not available')) {
      return 'Due to pruning strategies from validators, please navigate to the Settings tab and change your RPC node for your validator to retrieve your delegations.';
    }
    return message;
  };

  const pendingKey = (item: any, suffix: string) =>
    `pending-${item.event?.mix_id}-${item.event?.address ?? ''}-${item.event?.kind ?? ''}-${
      item.node_identity ?? ''
    }-${suffix}`;

  const nextEpochLine =
    nextEpoch instanceof Error || !nextEpoch ? null : (
      <Typography fontSize={14} color="text.secondary" sx={{ lineHeight: 1.5 }}>
        Next epoch starts at <strong>{nextEpoch}</strong>
      </Typography>
    );

  const emptyTableMessage = searchNeedle ? 'No delegations match your search.' : 'No delegations to show.';

  return (
    <>
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

      <Stack spacing={2} sx={{ width: '100%' }}>
        <Stack spacing={2}>
          <Box sx={{ maxWidth: 800 }}>
            <Typography variant="h6" component="h2">
              Delegations
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
              {activeCount} active
            </Typography>
          </Box>

          <Stack spacing={1.25}>
            <Stack
              direction={{ xs: 'column', md: 'row' }}
              spacing={2}
              alignItems={{ xs: 'stretch', md: 'flex-end' }}
              flexWrap="wrap"
            >
              <TextField
                size="small"
                label="Search identity"
                value={identityFilter}
                onChange={(e) => setIdentityFilter(e.target.value)}
                sx={{
                  minWidth: { xs: '100%', md: 220 },
                  flex: { md: '1 1 200px' },
                  '& .MuiOutlinedInput-root': { borderRadius: 2 },
                }}
              />
              <FormControl size="small" sx={{ minWidth: 200, '& .MuiOutlinedInput-root': { borderRadius: 2 } }}>
                <InputLabel id="delegation-sort-field-label">Sort by</InputLabel>
                <Select
                  labelId="delegation-sort-field-label"
                  label="Sort by"
                  value={orderBy}
                  onChange={(e) => setOrderBy(e.target.value as SortingKeys)}
                >
                  {SORT_FIELD_OPTIONS.map((opt) => (
                    <MenuItem key={opt.id} value={opt.id}>
                      {opt.label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
              <FormControl size="small" sx={{ minWidth: 160, '& .MuiOutlinedInput-root': { borderRadius: 2 } }}>
                <InputLabel id="delegation-sort-order-label">Order</InputLabel>
                <Select
                  labelId="delegation-sort-order-label"
                  label="Order"
                  value={order}
                  onChange={(e) => setOrder(e.target.value as Order)}
                >
                  <MenuItem value="asc">Ascending</MenuItem>
                  <MenuItem value="desc">Descending</MenuItem>
                </Select>
              </FormControl>
            </Stack>
            {nextEpochLine ? <Box sx={{ pt: 0.25 }}>{nextEpochLine}</Box> : null}
          </Stack>
        </Stack>

        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2} alignItems="stretch">
          <Box
            sx={{
              flex: 1,
              minWidth: 0,
              minHeight: 92,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              borderRadius: 2,
              border: (t) => `1px solid ${t.palette.divider}`,
              bgcolor: (t) =>
                t.palette.mode === 'dark' ? 'nym.nymWallet.nav.background' : 'nym.nymWallet.background.subtle',
              p: 2.5,
            }}
          >
            <Stack direction="row" alignItems="center" gap={0.5}>
              <InfoTooltip title="The total amount you have delegated to node(s) in the network. The amount also includes the rewards you have accrued since last time you claimed your rewards" />
              <Typography variant="body2" color="text.secondary">
                Total delegations
              </Typography>
            </Stack>
            <Typography fontWeight={600} fontSize={16} sx={{ mt: 0.5, textTransform: 'uppercase' }}>
              {delegationsSummaryLoading ? <Skeleton width={140} height={22} /> : totalDelegationsAndRewards ?? '-'}
            </Typography>
          </Box>
          <Box
            sx={{
              flex: 1,
              minWidth: 0,
              minHeight: 92,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              borderRadius: 2,
              border: (t) => `1px solid ${t.palette.divider}`,
              bgcolor: (t) =>
                t.palette.mode === 'dark' ? 'nym.nymWallet.nav.background' : 'nym.nymWallet.background.subtle',
              p: 2.5,
            }}
          >
            <Stack direction="row" alignItems="center" gap={0.5}>
              <InfoTooltip title="The initial amount you delegated to the node(s)" />
              <Typography variant="body2" color="text.secondary">
                Original delegations
              </Typography>
            </Stack>
            <Typography fontWeight={600} fontSize={16} sx={{ mt: 0.5, textTransform: 'uppercase' }}>
              {delegationsSummaryLoading ? <Skeleton width={120} height={22} /> : totalDelegations ?? '-'}
            </Typography>
          </Box>
          <Box
            sx={{
              flex: 1,
              minWidth: 0,
              minHeight: 92,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              borderRadius: 2,
              border: (t) => `1px solid ${t.palette.divider}`,
              bgcolor: (t) =>
                t.palette.mode === 'dark' ? 'nym.nymWallet.nav.background' : 'nym.nymWallet.background.subtle',
              p: 2.5,
            }}
          >
            <Stack direction="row" alignItems="center" gap={0.5}>
              <InfoTooltip title="The rewards you have accrued since the last time you claimed your rewards. Rewards are automatically compounded. You can claim your rewards at any time." />
              <Typography variant="body2" color="text.secondary">
                Total rewards
              </Typography>
            </Stack>
            <Typography fontWeight={600} fontSize={16} sx={{ mt: 0.5, textTransform: 'uppercase' }}>
              {delegationsSummaryLoading ? <Skeleton width={120} height={22} /> : totalRewards ?? '-'}
            </Typography>
          </Box>
        </Stack>

        {pendingItems.length > 0 && (
          <Stack spacing={1}>
            <Typography variant="subtitle2" color="text.secondary">
              Pending
            </Typography>
            <Stack spacing={2}>
              {pendingItems.map((item: any, index: number) => {
                if (
                  item.event &&
                  item.event.kind === 'Delegate' &&
                  (!item.node_identity || item.node_identity === '')
                ) {
                  return (
                    <PendingDelegationCard
                      key={pendingKey(item, `d-${index}`)}
                      item={{
                        ...item,
                        node_identity: `Mix Identity Key ${item.event.mix_id}`,
                      }}
                      explorerUrl={explorerUrl}
                    />
                  );
                }

                return (
                  <PendingDelegationCard key={pendingKey(item, `p-${index}`)} item={item} explorerUrl={explorerUrl} />
                );
              })}
            </Stack>
          </Stack>
        )}

        <TableContainer
          sx={{
            width: '100%',
            overflowX: 'auto',
            borderRadius: 3,
            border: (t) => `1px solid ${t.palette.divider}`,
            bgcolor: (t) =>
              t.palette.mode === 'dark' ? 'nym.nymWallet.nav.background' : 'nym.nymWallet.background.subtle',
          }}
        >
          <ErrorModal
            open={Boolean(delegationItemErrors)}
            title={`Delegation errors for Node ID ${delegationItemErrors?.nodeId || 'unknown'}`}
            message={
              delegationItemErrors?.errors
                ? formatErrorMessage(delegationItemErrors.errors)
                : 'An unknown error occurred'
            }
            onClose={() => setDelegationItemErrors(undefined)}
          />
          <Table stickyHeader size="small" sx={{ tableLayout: 'fixed' }}>
            <TableHead>
              <TableRow>
                <TableCell sx={{ fontWeight: 600, py: 1.25, width: '40%' }}>Node</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600, py: 1.25, width: '16%' }}>
                  Amount
                </TableCell>
                <TableCell align="right" sx={{ fontWeight: 600, py: 1.25, width: '14%' }}>
                  Saturation
                </TableCell>
                <TableCell align="right" sx={{ fontWeight: 600, py: 1.25, width: '18%' }}>
                  Reward
                </TableCell>
                <TableCell align="right" sx={{ fontWeight: 600, py: 1.25, width: 120, minWidth: 112 }}>
                  Actions
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {displayedDelegations.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={5} sx={{ py: 1.25 }}>
                    <Typography variant="body2" color="text.secondary" sx={{ py: 1 }}>
                      {emptyTableMessage}
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                displayedDelegations.map((item) => {
                  const rowKey = `${item.mix_id}-${item.node_identity}`;
                  const isOpen = expandedKey === rowKey;
                  const nodeIsUnbonded = Boolean(!item.node_identity);
                  const satNum = saturationNumeric(item);
                  let satColor: 'text.secondary' | 'error.main' | 'success.main' = 'text.secondary';
                  if (satNum !== undefined) {
                    satColor = satNum > 1 ? 'error.main' : 'success.main';
                  }

                  const operatingCost = item.cost_params?.interval_operating_cost;
                  const uptime = item.avg_uptime_percent;
                  const routingDisplay = uptime != null && String(uptime) !== '-' ? `${uptime}%` : '-';
                  const marginDisplay = item.cost_params?.profit_margin_percent
                    ? `${toPercentIntegerString(item.cost_params.profit_margin_percent)}%`
                    : '-';
                  const costDisplay = operatingCost ? `${operatingCost.amount} ${operatingCost.denom}` : '-';
                  const delegatedDisplay = item.delegated_on_iso_datetime
                    ? format(new Date(item.delegated_on_iso_datetime), 'dd/MM/yyyy')
                    : '-';

                  const unbondedTooltip =
                    'This node has unbonded and it does not exist anymore. You need to undelegate from it to get your stake and outstanding rewards (if any) back.';

                  return (
                    <React.Fragment key={rowKey}>
                      <TableRow hover sx={{ '& > *': { borderBottom: 'unset' } }}>
                        <TableCell sx={{ py: 1.25, verticalAlign: 'middle' }}>
                          <Stack direction="row" alignItems="center" spacing={0.5}>
                            <IconButton
                              aria-label="expand row"
                              size="small"
                              onClick={() => setExpandedKey(isOpen ? null : rowKey)}
                            >
                              {isOpen ? <KeyboardArrowUp /> : <KeyboardArrowDown />}
                            </IconButton>
                            <Stack direction="row" alignItems="center" gap={0.5} flexWrap="wrap" sx={{ minWidth: 0 }}>
                              {item.errors && (
                                <Tooltip title="Open to view a list of errors that occurred">
                                  <IconButton
                                    size="small"
                                    onClick={() =>
                                      setDelegationItemErrors({ nodeId: item.node_identity, errors: item.errors! })
                                    }
                                  >
                                    <WarningAmberOutlined color="warning" fontSize="small" />
                                  </IconButton>
                                </Tooltip>
                              )}
                              {item.uses_vesting_contract_tokens && (
                                <Tooltip title="Delegation uses locked tokens">
                                  <LockOutlined sx={{ color: 'text.secondary', fontSize: 18 }} />
                                </Tooltip>
                              )}
                              {nodeIsUnbonded ? (
                                <Tooltip title={unbondedTooltip} arrow>
                                  <Typography color="text.secondary">-</Typography>
                                </Tooltip>
                              ) : (
                                <Link
                                  target="_blank"
                                  href={`${explorerUrl}/nodes/${item.mix_id}`}
                                  text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
                                  color="text.primary"
                                  noIcon
                                />
                              )}
                            </Stack>
                          </Stack>
                        </TableCell>
                        <TableCell align="right" sx={{ py: 1.25, whiteSpace: 'nowrap', verticalAlign: 'middle' }}>
                          {item.amount.amount} {item.amount.denom}
                        </TableCell>
                        <TableCell
                          align="right"
                          sx={{ py: 1.25, color: satColor, whiteSpace: 'nowrap', verticalAlign: 'middle' }}
                        >
                          {getStakeSaturation(item)}
                        </TableCell>
                        <TableCell align="right" sx={{ py: 1.25, whiteSpace: 'nowrap', verticalAlign: 'middle' }}>
                          {getRewardValue(item)}
                        </TableCell>
                        <TableCell align="right" sx={{ py: 1.25, whiteSpace: 'nowrap', verticalAlign: 'middle' }}>
                          {!item.pending_events.length && !nodeIsUnbonded && (
                            <DelegationsActionsMenu
                              onActionClick={(action) =>
                                onItemActionClick ? onItemActionClick(item, action) : undefined
                              }
                              disableRedeemingRewards={!item.unclaimed_rewards || item.unclaimed_rewards.amount === '0'}
                              disableDelegateMore={item.mixnode_is_unbonding}
                            />
                          )}
                          {!item.pending_events.length && nodeIsUnbonded && (
                            <IconButton sx={{ color: (t) => t.palette.nym.nymWallet.text.main }} size="small">
                              <Undelegate
                                onClick={() => (onItemActionClick ? onItemActionClick(item, 'undelegate') : undefined)}
                              />
                            </IconButton>
                          )}
                          {item.pending_events.length > 0 && (
                            <Tooltip
                              title="Your changes will take effect when the new epoch starts. There is a new epoch every hour."
                              arrow
                              componentsProps={{
                                tooltip: {
                                  sx: { textAlign: 'left' },
                                },
                              }}
                            >
                              <Typography variant="caption" color="text.secondary">
                                Pending events
                              </Typography>
                            </Tooltip>
                          )}
                        </TableCell>
                      </TableRow>
                      <TableRow>
                        <TableCell style={{ paddingBottom: 0, paddingTop: 0 }} colSpan={5}>
                          <Collapse in={isOpen} timeout="auto" unmountOnExit>
                            <Box
                              sx={{
                                py: 2,
                                px: 2,
                                borderRadius: 3,
                                overflow: 'hidden',
                                border: (t) =>
                                  `1px solid ${alpha(t.palette.divider, t.palette.mode === 'dark' ? 0.35 : 0.5)}`,
                                bgcolor: (t) =>
                                  t.palette.mode === 'dark'
                                    ? alpha(t.palette.common.white, 0.04)
                                    : alpha(t.palette.common.black, 0.03),
                              }}
                            >
                              <Table size="small" sx={{ tableLayout: 'fixed' }}>
                                <TableHead>
                                  <TableRow>
                                    <TableCell sx={{ fontWeight: 600, color: 'text.secondary', py: 1, width: '25%' }}>
                                      Routing
                                    </TableCell>
                                    <TableCell sx={{ fontWeight: 600, color: 'text.secondary', py: 1, width: '25%' }}>
                                      Margin
                                    </TableCell>
                                    <TableCell sx={{ fontWeight: 600, color: 'text.secondary', py: 1, width: '25%' }}>
                                      NYM cost
                                    </TableCell>
                                    <TableCell sx={{ fontWeight: 600, color: 'text.secondary', py: 1, width: '25%' }}>
                                      Delegated on
                                    </TableCell>
                                  </TableRow>
                                </TableHead>
                                <TableBody>
                                  <TableRow>
                                    <TableCell sx={{ py: 1.25, verticalAlign: 'top', borderBottom: 'none' }}>
                                      <Typography variant="body2" color="text.primary">
                                        {routingDisplay}
                                      </Typography>
                                    </TableCell>
                                    <TableCell sx={{ py: 1.25, verticalAlign: 'top', borderBottom: 'none' }}>
                                      <Typography variant="body2" color="text.primary">
                                        {marginDisplay}
                                      </Typography>
                                    </TableCell>
                                    <TableCell sx={{ py: 1.25, verticalAlign: 'top', borderBottom: 'none' }}>
                                      <Typography variant="body2" color="text.primary">
                                        {costDisplay}
                                      </Typography>
                                    </TableCell>
                                    <TableCell sx={{ py: 1.25, verticalAlign: 'top', borderBottom: 'none' }}>
                                      <Typography variant="body2" color="text.primary">
                                        {delegatedDisplay}
                                      </Typography>
                                    </TableCell>
                                  </TableRow>
                                </TableBody>
                              </Table>
                            </Box>
                          </Collapse>
                        </TableCell>
                      </TableRow>
                    </React.Fragment>
                  );
                })
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Stack>
    </>
  );
};
