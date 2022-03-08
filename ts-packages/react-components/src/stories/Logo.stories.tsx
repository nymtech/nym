import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymLogo, NymWordmark } from '../components';

export default {
  title: 'Logo/Nym Logo',
  component: NymLogo,
} as ComponentMeta<typeof NymLogo>;

export function Logo() {
  return <NymLogo height={250} />;
}

export function Wordmark() {
  return <NymWordmark height={250} />;
}
