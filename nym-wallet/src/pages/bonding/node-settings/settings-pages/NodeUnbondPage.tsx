import React, { useState } from 'react';
import { Box, Button, Typography, Grid, TextField } from '@mui/material';
import { TBondedMixnode, TBondedGateway } from 'src/context/bonding';
import { Error } from 'src/components/Error';
import { UnbondModal } from 'src/components/Bonding/modals/UnbondModal';
interface Props {
  bondedNode: TBondedMixnode | TBondedGateway;
  onConfirm: () => Promise<void>;
  onError: (e: string) => void;
}
export const NodeUnbondPage = ({ bondedNode, onConfirm, onError }: Props) => {
  const [confirmField, setConfirmField] = useState('');
  const [isConfirmed, setIsConfirmed] = useState(false);
  //TODO: Check what happens with a gateway
  return (
    <Box sx={{ pl: 3, minHeight: '450px' }}>
      <Grid container direction="row" alignItems="start" justifyContent={'space-between'}>
        <Grid item container direction={'column'} width={0.5} spacing={1} sx={{ pt: 3 }}>
          <Grid item>
            <Typography variant="body1" fontWeight={600}>
              Unbond
            </Typography>
          </Grid>
          <Grid item>
            <Typography variant="body2" sx={{ color: (theme) => theme.palette.nym.text.muted }}>
              If you unbond you will loose all delegations!
            </Typography>
          </Grid>
        </Grid>
        <Grid item container direction={'column'} spacing={2} width={0.5} padding={3}>
          <Grid item>
            <Error message="Unbonding is irreversible and it won’t be possible to restore the current state of your node again" />
          </Grid>
          <Grid item>
            <Typography variant="body2">
              To unbond, type{' '}
              <Typography display="inline" component="span" sx={{ color: (t) => t.palette.nym.highlight }}>
                UNBOND
              </Typography>{' '}
              in the field below and click continue
            </Typography>
          </Grid>
          <Grid item>
            <TextField fullWidth value={confirmField} onChange={(e) => setConfirmField(e.target.value)} />
          </Grid>
          <Grid item sx={{ mt: 2 }}>
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
          </Grid>
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
