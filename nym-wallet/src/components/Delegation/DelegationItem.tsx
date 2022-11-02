import React from 'react';
import { Chip, IconButton, TableCell, TableCellProps, TableRow, Tooltip, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { decimalToPercentage, DelegationWithEverything } from '@nymproject/types';
import { DelegationListItemActions, DelegationsActionsMenu } from './DelegationActions';
import { isDelegation } from 'src/context/delegations';
import { toPercentIntegerString } from 'src/utils';
import { format } from 'date-fns';
import { Undelegate } from 'src/svg-icons';
import { Box } from '@mui/system';
import { identity } from 'lodash';

const getStakeSaturation = (item: DelegationWithEverything) =>
  !item.stake_saturation ? '-' : `${decimalToPercentage(item.stake_saturation)}%`;

const getRewardValue = (item: DelegationWithEverything) => {
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
  return (
    <Tooltip
      arrow
      title={
        nodeIsUnbonded
          ? 'This node has unbonded and it does not exist anymore. You need to undelegate from it to get your stake and outstanding rewards (if any) back.'
          : ''
      }
    >
      <TableRow key={item.node_identity} sx={{ color: !Boolean(item.node_identity) ? 'error.main' : 'inherit' }}>
        <TableCell sx={{ color: 'inherit' }}>
          {nodeIsUnbonded ? (
            '-'
          ) : (
            <Link
              target="_blank"
              href={`${explorerUrl}/network-components/mixnode/${item.node_identity}`}
              text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
              color="text.primary"
              noIcon
            />
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
        <TableCell sx={{ color: 'inherit' }}>{getStakeSaturation(item)}</TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          {format(new Date(item.delegated_on_iso_datetime), 'dd/MM/yyyy')}
        </TableCell>
        <TableCell sx={{ color: 'inherit' }}>
          <Typography style={{ textTransform: 'uppercase' }}>
            {isDelegation(item) && `${item.amount.amount} ${item.amount.denom}`}
          </Typography>
        </TableCell>
        <TableCell sx={{ textTransform: 'uppercase', color: 'inherit' }}>{getRewardValue(item)}</TableCell>
        <TableCell align="right" sx={{ color: 'inherit' }}>
          {!item.pending_events.length && !nodeIsUnbonded && (
            <DelegationsActionsMenu
              isPending={undefined}
              onActionClick={(action) => (onItemActionClick ? onItemActionClick(item, action) : undefined)}
              disableRedeemingRewards={!item.unclaimed_rewards || item.unclaimed_rewards.amount === '0'}
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
