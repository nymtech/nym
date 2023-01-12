import React from 'react';
import { WalletAddressFormField } from '@nymproject/react/account/WalletAddressFormField';
import { SxProps } from '@mui/system';
import { Paper, Stack, Button, Box } from '@mui/material';
import ArrowCircleRightIcon from '@mui/icons-material/ArrowCircleRight';
import { useTestAndEarnContext } from './context/TestAndEarnContext';

export const TestAndEarnEnterWalletAddress: FCWithChildren<{
  initialValue?: string;
  placeholder?: string;
  onSubmit?: () => Promise<void> | void;
  sx?: SxProps;
}> = ({ initialValue, placeholder, onSubmit, sx }) => {
  const context = useTestAndEarnContext();
  const [isAddressValid, setAddressIsValid] = React.useState(false);
  return (
    <Paper sx={{ py: 4, px: 2 }}>
      <Stack spacing={4}>
        <Box>
          <WalletAddressFormField
            label="Wallet address"
            initialValue={initialValue}
            placeholder={placeholder || 'Please enter your wallet address'}
            onChanged={context.setWalletAddress}
            onValidate={setAddressIsValid}
            sx={{ width: '80%' }}
          />
        </Box>
        <Box>
          <Button variant="contained" endIcon={<ArrowCircleRightIcon />} disabled={!isAddressValid} onClick={onSubmit}>
            Submit
          </Button>
        </Box>
      </Stack>
    </Paper>
  );
};
