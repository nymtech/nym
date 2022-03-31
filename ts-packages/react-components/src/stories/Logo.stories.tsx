import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymLogo, NymWordmark } from '../components';

export default {
  title: 'Logo/Nym Logo',
  component: NymLogo,
} as ComponentMeta<typeof NymLogo>;

export const Logo = () => <NymLogo height={250} />;

export const Wordmark = () => <NymWordmark height={250} />;
