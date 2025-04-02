import React from 'react';
import { Chip, TableCell, TableRow, Tooltip } from '@mui/material';
import { WrappedDelegationEvent } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';

export const PendingDelegationItem = ({ item, explorerUrl }: { item: WrappedDelegationEvent; explorerUrl: string }) => (
  <TableRow key={item.node_identity}>
    <TableCell>
      <Link
        target="_blank"
        href={`${explorerUrl}/nodes/${item.event.mix_id}`}
        text={`${item.node_identity.slice(0, 6)}...${item.node_identity.slice(-6)}`}
        color="text.primary"
        noIcon
      />
    </TableCell>
    <TableCell>-</TableCell>
    <TableCell>-</TableCell>
    <TableCell>-</TableCell>
    <TableCell>-</TableCell>
    <TableCell>-</TableCell>
    <TableCell>-</TableCell>
    <TableCell>-</TableCell>
    <TableCell align="right">
      <Tooltip
        title={`Your delegation of ${item.event.amount?.amount} ${item.event.amount?.denom} will take effect 
            when the new epoch starts. There is a new
            epoch every hour.`}
        arrow
      >
        <Chip label="Pending Events" />
      </Tooltip>
    </TableCell>
  </TableRow>
);
