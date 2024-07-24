import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { NymLogo } from '../../../lib/components/logo/NymLogo';
import { NymIcon } from '../../../lib/components/logo/NymIcon';
import { NymWordmark } from '../../../lib/components/logo/NymWordmark';

export default {
  title: 'Branding/Nym Logo',
  component: NymLogo,
} as ComponentMeta<typeof NymLogo>;

export const Logo = () => <NymLogo height={250} />;

export const Icon = () => <NymIcon height={250} />;

export const Wordmark = () => <NymWordmark height={250} />;
