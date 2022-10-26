import React from 'react';
import { Chip, IconButton, TableCell, TableCellProps, TableRow, Tooltip, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { decimalToPercentage, DelegationWithEverything } from '@nymproject/types';
import { DelegationListItemActions, DelegationsActionsMenu } from './DelegationActions';
import { isDelegation } from 'src/context/delegations';
import { toPercentIntegerString } from 'src/utils';
import { format } from 'date-fns';
import { Undelegate } from 'src/svg-icons';

const getStakeSaturation = (item: DelegationWithEverything) =>
  !item.stake_saturation ? '-' : `${decimalToPercentage(item.stake_saturation)}%`;

const getRewardValue = (item: DelegationWithEverything) => {
  const { unclaimed_rewards } = item;
  return !unclaimed_rewards ? '-' : `${unclaimed_rewards.amount} ${unclaimed_rewards.denom}`;
};

const WrappedTableCell = (props: TableCellProps & { withWarning?: boolean }) => (
  <TableCell {...props}>
    <Typography color={props.withWarning ? 'error.main' : 'inherit'}>{props.children}</Typography>
  </TableCell>
);

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
    <TableRow key={item.node_identity}>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)}>
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
      </WrappedTableCell>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)}>
        {isDelegation(item) && (!item.avg_uptime_percent ? '-' : `${item.avg_uptime_percent}%`)}
      </WrappedTableCell>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)}>
        {isDelegation(item) &&
          (!item.cost_params?.profit_margin_percent
            ? '-'
            : `${toPercentIntegerString(item.cost_params.profit_margin_percent)}%`)}
      </WrappedTableCell>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)}>{getStakeSaturation(item)}</WrappedTableCell>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)}>
        {format(new Date(item.delegated_on_iso_datetime), 'dd/MM/yyyy')}
      </WrappedTableCell>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)}>
        <Typography style={{ textTransform: 'uppercase' }}>
          {isDelegation(item) && `${item.amount.amount} ${item.amount.denom}`}
        </Typography>
      </WrappedTableCell>
      <WrappedTableCell withWarning={!Boolean(item.node_identity)} sx={{ textTransform: 'uppercase' }}>
        {getRewardValue(item)}
      </WrappedTableCell>
      <WrappedTableCell align="right">
        {!item.pending_events.length && !nodeIsUnbonded && (
          <DelegationsActionsMenu
            isPending={undefined}
            onActionClick={(action) => (onItemActionClick ? onItemActionClick(item, action) : undefined)}
            disableRedeemingRewards={!item.unclaimed_rewards || item.unclaimed_rewards.amount === '0'}
          />
        )}
        {!item.pending_events.length && nodeIsUnbonded && (
          <IconButton>
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
      </WrappedTableCell>
    </TableRow>
  );
};
