import React, { useState } from 'react';
import { Box, Button, FormHelperText, InputLabel, Stack, TextField, Typography } from '@mui/material';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { Node as NodeIcon } from 'src/svg-icons/node';
import { TBondedMixnode } from 'src/context';
import { Tabs } from 'src/components/Tabs';
import { ModalListItem } from 'src/components/Modals/ModalListItem';

export const NodeSettings = ({
  currentPm,
  onClose,
}: {
  currentPm: TBondedMixnode['profitMargin'];
  onClose: () => void;
}) => {
  const [pm, setPm] = useState(currentPm.toString());
  return (
    <SimpleModal
      open
      hideCloseIcon
      sx={{ p: 0 }}
      header={
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, p: 3 }}>
          <NodeIcon />
          <Typography variant="h6" fontWeight={600}>
            Node Settings
          </Typography>
        </Box>
      }
      okLabel="Next"
      onClose={onClose}
    >
      <Tabs tabs={['System variables']} selectedTab={0} />
      <Box sx={{ p: 3 }}>
        <Typography fontWeight={600} sx={{ mb: 1 }}>
          Set profit margin
        </Typography>
        <Box sx={{ mb: 3 }}>
          <TextField placeholder="Profit margin" value={pm} onChange={(e) => setPm(e.target.value)} fullWidth />
          <FormHelperText>Your new profit margin will be applied in the next epoch</FormHelperText>
        </Box>
        <Box sx={{ mb: 3 }}>
          <ModalListItem label="Estimated operator reward for 10% profit margin" value="150 NYM" divider />
          <ModalListItem label="Est. fee for this operation will be cauculated in the next page" value="" />
        </Box>
        <Button variant="contained" fullWidth size="large" onClick={() => {}}>
          Next
        </Button>
      </Box>
    </SimpleModal>
  );
};
