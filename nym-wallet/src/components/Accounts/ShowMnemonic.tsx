import React, { useEffect, useState } from 'react';
import { Box, Typography } from '@mui/material';
import { CopyToClipboard } from '@nymproject/react';

export const ShowMnemonic = ({ accountName }: { accountName: string }) => {
  const [showMnemonic, setShowMnemonic] = useState<string>();
  const [mnemonic, setMnemonic] = useState<string>();

  return (
    <Box>
      <Typography
        variant="body2"
        sx={{ textDecoration: 'underline', mb: 0.5 }}
        onClick={(e) => {
          e.stopPropagation();
          setShowMnemonic((show) => (!show ? accountName : undefined));
        }}
      >
        {`${showMnemonic ? 'Hide' : 'Show'} mnemonic`}
      </Typography>
      {mnemonic && (
        <Box display="flex" alignItems="end">
          <Typography variant="caption">{mnemonic}</Typography>
          <CopyToClipboard sx={{ width: 18 }} value={mnemonic} />
        </Box>
      )}
    </Box>
  );
};
