import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Box } from '@mui/material';
import { WalletAddressFormField } from '../../../lib/components/account/WalletAddressFormField';

export default {
  title: 'Accounts/Wallet Address',
  component: WalletAddressFormField,
} as ComponentMeta<typeof WalletAddressFormField>;

export const Empty = () => <WalletAddressFormField />;

export const ErrorValue = () => <WalletAddressFormField initialValue="this is a bad value" />;

export const ValidValue = () => <WalletAddressFormField initialValue="n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" />;

export const ReadOnlyValidValue = () => (
  <WalletAddressFormField readOnly initialValue="n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" />
);

export const ReadOnlyErrorValue = () => <WalletAddressFormField readOnly initialValue="this is a bad value" />;

export const WithLabel = () => (
  <Box p={2}>
    <WalletAddressFormField
      initialValue="n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec"
      textFieldProps={{ label: 'Identity Key' }}
    />
  </Box>
);

export const WithPlaceholder = () => (
  <WalletAddressFormField textFieldProps={{ placeholder: 'Please enter an wallet address' }} />
);

export const FullWidth = () => (
  <WalletAddressFormField fullWidth initialValue="n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" />
);

export const HideValidTick = () => (
  <WalletAddressFormField showTickOnValid={false} fullWidth initialValue="n1xr4w0kddak8d8zlfmu8sl6dk2r4p9uhhzzlaec" />
);
