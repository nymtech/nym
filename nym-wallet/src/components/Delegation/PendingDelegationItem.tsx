import React from 'react';
import { Box, Chip, TableCell, TableRow, Tooltip } from '@mui/material';
import { WrappedDelegationEvent } from '@nymproject/types';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';

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
    <TableCell>
      <Box sx={{ textAlign: 'left' }}>
        {item.event.amount?.amount} NYM
      </Box>
    </TableCell>
    <TableCell>-</TableCell>
    <TableCell sx={{ textAlign: 'center' }}>
      <Box sx={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
        <Tooltip
          title={
            <div style={{ textAlign: 'center', width: '100%' }}>
              Your delegation of {item.event.amount?.amount} {item.event.amount?.denom} will take effect 
              when the new epoch starts. There is a new
              epoch every hour.
            </div>
          }
          arrow
          PopperProps={{
            sx: {
              '& .MuiTooltip-tooltip': {
                textAlign: 'center'
              }
            }
          }}
        >
          <Chip label="Pending Events" />
        </Tooltip>
      </Box>
    </TableCell>
  </TableRow>
);