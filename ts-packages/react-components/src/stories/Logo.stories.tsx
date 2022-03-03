import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Box } from '@mui/material';
import { NymLogo, NymWordmark } from '../components';

export default {
  title: 'Logo/Nym Logo',
  component: NymLogo,
} as ComponentMeta<typeof NymLogo>;

export function Logo() {
  return <NymLogo height={250} />;
}

export function Wordmark() {
  return (
    <div style={{ background: '#888', padding: '2rem' }}>
      <NymWordmark height={250} />
    </div>
  );
}
