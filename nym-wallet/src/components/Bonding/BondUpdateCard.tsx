import React from 'react';
import { Box, Button, Stack, Tooltip, Typography } from '@mui/material';
import { NymCard } from 'src/components';

export const BondUpdateCard = ({ setSuccesfullUpdate }: { setSuccesfullUpdate: (staus: boolean) => void }) => (
  <Stack gap={2}>
    <NymCard
      borderless
      title={
        <Typography variant="h5" fontWeight={600} marginBottom={3}>
          Upgrade your node!
        </Typography>
      }
      subheader={
        <Stack gap={1}>
          <Typography variant="subtitle2" fontWeight={600} sx={{ color: 'nym.text.dark' }}>
            It seems like your node is running outdated binaries.
          </Typography>
          <Typography variant="body2">Update to the latest stable Nym node binary now*</Typography>
          <Typography variant="body2">The update takes less than a minute!</Typography>
          <Typography variant="caption">
            *Without updating, legacy node settings can be changed in the Nym CLI.
          </Typography>
        </Stack>
      }
      Action={
        <Box display="flex" flexDirection="column" alignItems="flex-end" justifyContent="space-between" height={70}>
          <Tooltip title="Update to the latest stable Nym node binary now">
            <Box>
              <Button
                variant="contained"
                color="primary"
                // TODO wallet-smoosh: update when we have the actual endpoint
                onClick={() => setSuccesfullUpdate(true)}
              >
                Upgrade to Nym Node
              </Button>
            </Box>
          </Tooltip>
        </Box>
      }
    />
  </Stack>
);
