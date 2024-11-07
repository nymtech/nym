import React, { useState } from 'react';
import { contracts } from '@nymproject/contract-clients';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import Table from '@mui/material/Table';
import { TableBody, TableCell, TableHead, TableRow } from '@mui/material';
import { settings } from './client';

const getClient = async () => {
  const cosmWasmClient = await SigningCosmWasmClient.connect(settings.url);

  const client = new contracts.Mixnet.MixnetQueryClient(cosmWasmClient, settings.mixnetContractAddress);
  return client;
};

export const Mixnodes = () => {
  const [mixnodes, setMixnodes] = useState<any>();
  const [busy, setBusy] = useState<boolean>(false);

  const getMixnodes = async () => {
    setBusy(true);
    const client = await getClient();
    const { nodes } = await client.getMixNodesDetailed({});

    setMixnodes(nodes);
    setBusy(false);
  };

  if (busy) {
    return (
      <Box pt={4}>
        <Stack direction="row" spacing={2} alignItems="center">
          <CircularProgress />
          <Typography>Loading...</Typography>
        </Stack>
      </Box>
    );
  }

  if (!mixnodes) {
    return (
      <Box pt={4}>
        <Button variant="outlined" onClick={getMixnodes}>
          Query for mixnodes
        </Button>
      </Box>
    );
  }

  return (
    <Box pt={4}>
      {mixnodes?.length && (
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>MixId</TableCell>
              <TableCell>Owner Account</TableCell>
              <TableCell>Layer</TableCell>
              <TableCell>Bonded at Block Height</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {mixnodes.map((mixnode: any) => (
              <TableRow key={mixnode.bond_information.mix_id}>
                <TableCell>{mixnode.bond_information.mix_id}</TableCell>
                <TableCell>{mixnode.bond_information.owner}</TableCell>
                <TableCell>{mixnode.bond_information.layer}</TableCell>
                <TableCell>{mixnode.bond_information.bonding_height}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </Box>
  );
};
