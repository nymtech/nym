import React, { useState } from 'react';
import { Box, Button, Typography, Grid, TextField, Stack } from '@mui/material';
import { TBondedMixnode, TBondedGateway } from '@src/context/bonding';
import { Error } from '@src/components/Error';
import { UnbondModal } from '@src/components/Bonding/modals/UnbondModal';
import { isMixnode } from '@src/types';

interface Props {
  bondedNode: TBondedMixnode | TBondedGateway;

  onConfirm: () => Promise<void>;
  onError: (e: string) => void;
}
export const NodeUnbondPage = ({ bondedNode, onConfirm, onError }: Props) => {
  const [confirmField, setConfirmField] = useState('');
  const [isConfirmed, setIsConfirmed] = useState(false);
  // TODO: Check what happens with a gateway
  return (
    <Box sx={{ p: 3, minHeight: '450px' }}>
      <Grid container justifyContent="space-between">
        <Grid item xs={12} lg={4} sx={{ mb: 3 }}>
          <Stack gap={1}>
            <Typography variant="body1" fontWeight={600}>
              Unbond
            </Typography>

            {isMixnode(bondedNode) && (
              <Grid item>
                <Typography variant="body2" sx={{ color: (theme) => theme.palette.nym.text.muted }}>
                  Remember you should only unbond if you want to remove your node from the network for good.
                </Typography>
              </Grid>
            )}
          </Stack>
        </Grid>
        <Grid item xs={12} lg={6}>
          <Stack gap={3}>
            {isMixnode(bondedNode) && (
              <Error message="Unbonding is irreversible. You will lose all your delegations. It won’t be possible to restore the current state of your node again." />
            )}

            <Typography variant="body2">
              To unbond, type{' '}
              <Typography display="inline" component="span" sx={{ color: (t) => t.palette.nym.highlight }}>
                UNBOND
              </Typography>{' '}
              in the field below and click continue
            </Typography>

            <TextField
              fullWidth
              value={confirmField}
              onChange={(e) => setConfirmField(e.target.value)}
              InputLabelProps={{ shrink: true }}
            />

            <Button
              size="large"
              variant="contained"
              fullWidth
              disabled={confirmField !== 'UNBOND'}
              onClick={() => {
                setIsConfirmed(true);
              }}
            >
              Continue
            </Button>
          </Stack>
        </Grid>
      </Grid>
      {isConfirmed && (
        <UnbondModal
          node={bondedNode}
          onConfirm={async () => {
            setIsConfirmed(false);
            onConfirm();
          }}
          onClose={() => setIsConfirmed(false)}
          onError={onError}
        />
      )}
    </Box>
  );
};
