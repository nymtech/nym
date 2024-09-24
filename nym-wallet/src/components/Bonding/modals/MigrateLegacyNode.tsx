import React from 'react';
import { Box, Typography } from '@mui/material';
import { SimpleModal } from 'src/components/Modals/SimpleModal';

const MigrateLegacyNode = ({
  open,
  onClose,
  handleMigrate,
}: {
  open: boolean;
  onClose: () => void;
  handleMigrate: () => Promise<void>;
}) => (
  <SimpleModal
    open={open}
    header={<Typography sx={{ fontWeight: 700 }}>Migrate Legacy Node</Typography>}
    onOk={handleMigrate}
    okLabel="Migrate"
    onClose={onClose}
    sx={{ maxWidth: 500 }}
  >
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Typography>You have a legacy node that needs to be migrated to the new system.</Typography>
    </Box>
  </SimpleModal>
);

export default MigrateLegacyNode;
