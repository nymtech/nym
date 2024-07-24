import { Link } from '@nymproject/react';
import { Box, Button, Typography } from '@mui/material';
import { NymCard } from '../NymCard';

export const Bond = ({
  onBond,
  disabled,
}: {
  onBond: () => void;

  disabled: boolean;
}) => (
  <NymCard title="Bonding" borderless>
    <Box
      sx={{
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'space-between',
      }}
    >
      <Typography variant="body2">
        Bond a mix node or a gateway. Learn how to set up and run a node{' '}
        <Link href="https://nymtech.net/operators/nodes/mix-node-setup.html" target="_blank">
          here
        </Link>
      </Typography>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'flex-end',
          justifyContent: 'space-between',
          gap: 2,
        }}
      >
        <Button
          size="large"
          variant="contained"
          color="primary"
          type="button"
          disableElevation
          onClick={onBond}
          disabled={disabled}
        >
          Bond
        </Button>
      </Box>
    </Box>
  </NymCard>
);
