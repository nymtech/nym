import React, { useContext } from 'react';
import { Table, TableCell, TableHead, TableRow } from '@mui/material';
import { DelegationResult } from '@nymproject/types';
import { AppContext } from 'src/context';

export const PendingEvents = ({
  pendingDelegations,
  show,
}: {
  pendingDelegations: DelegationResult[];
  show: boolean;
}) => {
  const { clientDetails } = useContext(AppContext);

  return show ? (
    <Table>
      <TableHead>
        <TableRow>
          <TableCell sx={{ pl: 3 }}>Address</TableCell>
          <TableCell>Amount</TableCell>
        </TableRow>
      </TableHead>
      {pendingDelegations.map((delegation) => (
        <TableRow>
          <TableCell sx={{ maxWidth: 200, pl: 3 }}>{delegation.target_address}</TableCell>
          <TableCell align="left">{`${delegation.amount?.amount} ${clientDetails?.denom}`}</TableCell>
        </TableRow>
      ))}
    </Table>
  ) : null;
};
