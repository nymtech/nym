import React, { useCallback, useContext, useEffect, useState } from 'react';
import { Table, TableCell, TableHead, TableRow } from '@mui/material';
import { minorToMajor } from 'src/requests';
import { DelegationResult } from 'src/types';
import { AppContext } from 'src/context';

export const PendingEvents = ({
  pendingDelegations,
  show,
}: {
  pendingDelegations: DelegationResult[];
  show: boolean;
}) => {
  const [mapped, setMapped] = useState<Array<DelegationResult & { majorValue: string }>>([]);
  const { currency } = useContext(AppContext);

  const mapToMajorValue = useCallback(async () => {
    const mappedToMajor = await Promise.all(
      pendingDelegations.map(async (pendingDelegation) => {
        const majorValue = await minorToMajor(pendingDelegation.amount?.amount || '');
        return { ...pendingDelegation, majorValue: majorValue.amount };
      }),
    );
    setMapped(mappedToMajor);
  }, [pendingDelegations]);

  useEffect(() => {
    mapToMajorValue();
  }, []);
  return show ? (
    <Table>
      <TableHead>
        <TableRow>
          <TableCell sx={{ pl: 3 }}>Address</TableCell>
          <TableCell>Amount</TableCell>
        </TableRow>
      </TableHead>
      {mapped.map((delegation) => (
        <TableRow>
          <TableCell sx={{ maxWidth: 200, pl: 3 }}>{delegation.target_address}</TableCell>
          <TableCell align="left">{`${delegation.majorValue} ${currency?.major}`}</TableCell>
        </TableRow>
      ))}
    </Table>
  ) : null;
};
