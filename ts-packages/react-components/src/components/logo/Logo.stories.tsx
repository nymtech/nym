import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymLogo } from './NymLogo';
import { NymWordmark } from './NymWordmark';
import { NymIcon } from './NymIcon';

export default {
  title: 'Branding/Nym Logo',
  component: NymLogo,
} as ComponentMeta<typeof NymLogo>;

export const Logo = () => <NymLogo height={250} />;

export const Icon = () => <NymIcon height={250} />;

export const Wordmark = () => <NymWordmark height={250} />;
