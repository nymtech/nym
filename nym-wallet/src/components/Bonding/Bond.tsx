import React from 'react';
import { TauriLink as Link } from 'src/components/TauriLinkWrapper';
import { Box, Button, Typography } from '@mui/material';
import { NymCard } from '../NymCard';

export const Bond = ({
  onBond,
  disabled,
}: {
  onBond: () => void;

  disabled: boolean;
}) => (
  <NymCard hideHeader borderless dataTestid="bond-run-node">
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        textAlign: 'center',
        gap: 3,
      }}
    >
      <Box sx={{ width: '100%' }}>
        <Typography
          variant="overline"
          sx={{
            display: 'block',
            color: 'text.secondary',
            letterSpacing: '0.12em',
            fontWeight: 600,
            mb: 0.75,
          }}
        >
          Operator
        </Typography>
        <Typography component="h2" variant="h5" sx={{ fontWeight: 700, lineHeight: 1.25 }}>
          Run a node
        </Typography>
      </Box>
      <Typography
        variant="body2"
        sx={{
          color: 'text.secondary',
          maxWidth: 640,
          mx: 'auto',
          lineHeight: 1.6,
        }}
      >
        Bonding locks NYM as pledge so you can run a nym node on the network. You will need enough liquid NYM to cover
        the minimum pledge and fees. Read the{' '}
        <Link href="https://nym.com/docs/operators/nodes/nym-node" target="_blank">
          node setup and bonding guide
        </Link>{' '}
        before you continue.
      </Typography>
      <Button
        size="large"
        variant="contained"
        color="primary"
        type="button"
        disableElevation
        onClick={onBond}
        disabled={disabled}
        sx={{
          alignSelf: 'stretch',
          maxWidth: 360,
          width: '100%',
          mx: 'auto',
        }}
      >
        Bond
      </Button>
    </Box>
  </NymCard>
);
