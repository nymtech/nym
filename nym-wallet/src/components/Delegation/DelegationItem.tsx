import React from 'react';
import { Box, Chip, IconButton, TableCell, TableRow, Tooltip, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { decimalToPercentage, DelegationWithEverything } from '@nymproject/types';
import { LockOutlined, WarningAmberOutlined } from '@mui/icons-material';
import { isDelegation, useDelegationContext } from 'src/context/delegations';
import { toPercentIntegerString } from 'src/utils';
import { format } from 'date-fns';
import { Undelegate } from 'src/svg-icons';
import { DelegationListItemActions, DelegationsActionsMenu } from './DelegationActions';

const getStakeSaturation = (item: DelegationWithEverything) =>
  !item.stake_saturation ? '-' : `${decimalToPercentage(item.stake_saturation)}%`;

const getRewardValue = (item: DelegationWithEverything) => {
  // eslint-disable-next-line @typescript-eslint/naming-convention
  const { unclaimed_rewards } = item;
  return !unclaimed_rewards ? '-' : `${unclaimed_rewards.amount} ${unclaimed_rewards.denom}`;
};

export const DelegationItem = ({
  item,
  explorerUrl,
  nodeIsUnbonded,
  onItemActionClick,
}: {
  item: DelegationWithEverything;
  explorerUrl: string;
  nodeIsUnbonded: boolean;
  onItemActionClick?: (item: DelegationWithEverything, action: DelegationListItemActions) => void;
}) => {
  const { setDelegationItemErrors } = useDelegationContext();

  const operatingCost = isDelegation(item) && item.cost_params?.interval_operating_cost;

  const tooltipText = () => {
    if (nodeIsUnbonded) {
      return 'This node has unbonded and it does not exist anymore. You need to undelegate from it to get your stake and outstanding rewards (if any) back.';
    }
    return '';
  };

  return (
    <Tooltip arrow title={tooltipText()}>
      <TableRow key={item.node_identity} sx={{ color: !item.node_identity ? 'error.main' : 'inherit' }}>
        <TableCell sx={{ color: 'inherit', pr: 1 }} padding="normal">
          {nodeIsUnbonded ? (
            '-'
          ) : (
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              {item.errors && (
                <Tooltip title="Open to view a list of errors that occurred">
                  <IconButton
                    sx={{ mr: 1 }}
                    size="small"
                    onClick={() => setDelegationItemErrors({ nodeId: item.node_identity, errors: item.errors! })}
                  >
                    <WarningAmberOutlined color="warning" fontSize="small" />
                  </IconButton>
                </Tooltip>
              )}
              <Link
                target="_blank"
                href={`${explorerUrl}/network-components/mixnode/${item.mix_id}`}
                text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
                color="text.primary"
                noIcon
              />
            </Box>
          )}
        </TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          {isDelegation(item) && (!item.avg_uptime_percent ? '-' : `${item.avg_uptime_percent}%`)}
        </TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          {isDelegation(item) &&
            (!item.cost_params?.profit_margin_percent
              ? '-'
              : `${toPercentIntegerString(item.cost_params.profit_margin_percent)}%`)}
        </TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          <Typography style={{ textTransform: 'uppercase', fontSize: 'inherit' }}>
            {operatingCost ? `${operatingCost.amount} ${operatingCost.denom}` : '-'}
          </Typography>
        </TableCell>
        <TableCell sx={{ color: 'inherit' }}>{getStakeSaturation(item)}</TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          {item.delegated_on_iso_datetime && format(new Date(item.delegated_on_iso_datetime), 'dd/MM/yyyy')}
        </TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          <Typography style={{ textTransform: 'uppercase', fontSize: 'inherit' }}>
            {isDelegation(item) ? `${item.amount.amount} ${item.amount.denom}` : '-'}
          </Typography>
        </TableCell>
        <TableCell sx={{ textTransform: 'uppercase', color: 'inherit' }}>{getRewardValue(item)}</TableCell>
        <TableCell>
          {item.uses_vesting_contract_tokens && (
            <Tooltip title="Delegation uses locked tokens">
              <LockOutlined sx={{ color: 'grey.800' }} fontSize="small" />
            </Tooltip>
          )}
        </TableCell>
        <TableCell align="right" sx={{ color: 'inherit' }}>
          {!item.pending_events.length && !nodeIsUnbonded && (
            <DelegationsActionsMenu
              onActionClick={(action) => (onItemActionClick ? onItemActionClick(item, action) : undefined)}
              disableRedeemingRewards={!item.unclaimed_rewards || item.unclaimed_rewards.amount === '0'}
              disableDelegateMore={item.mixnode_is_unbonding}
            />
          )}
          {!item.pending_events.length && nodeIsUnbonded && (
            <IconButton sx={{ color: (t) => t.palette.nym.nymWallet.text.main }}>
              <Undelegate onClick={() => (onItemActionClick ? onItemActionClick(item, 'undelegate') : undefined)} />
            </IconButton>
          )}
          {item.pending_events.length > 0 && (
            <Tooltip
              title="Your changes will take effect when the new epoch starts. There is a new epoch every hour."
              arrow
            >
              <Chip label="Pending Events" />
            </Tooltip>
          )}
        </TableCell>
      </TableRow>
    </Tooltip>
  );
};
