import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Box } from '@mui/material';
import { IdentityKeyFormField } from './IdentityKeyFormField';

export default {
  title: 'Mixnodes/Identity Key',
  component: IdentityKeyFormField,
} as ComponentMeta<typeof IdentityKeyFormField>;

export const Empty = () => <IdentityKeyFormField />;

export const ErrorValue = () => <IdentityKeyFormField initialValue="this is a bad value" />;

export const ValidValue = () => <IdentityKeyFormField initialValue="DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" />;

export const ReadOnlyValidValue = () => (
  <IdentityKeyFormField readOnly initialValue="DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" />
);

export const ReadOnlyErrorValue = () => <IdentityKeyFormField readOnly initialValue="this is a bad value" />;

export const WithLabel = () => (
  <Box p={2}>
    <IdentityKeyFormField
      initialValue="DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu"
      textFieldProps={{ label: 'Identity Key' }}
    />
  </Box>
);

export const WithPlaceholder = () => (
  <IdentityKeyFormField textFieldProps={{ placeholder: 'Please enter an Identity Key' }} />
);

export const FullWidth = () => (
  <IdentityKeyFormField fullWidth initialValue="DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" />
);

export const HideValidTick = () => (
  <IdentityKeyFormField showTickOnValid={false} fullWidth initialValue="DZ6RfeY8DttMD3CQKoayV6mss5a5FC3RoH75Kmcujyhu" />
);
