import { ComponentMeta } from '@storybook/react';
import { NymLogo, NymWordmark, NymIcon } from '@lib/components/logo';

export default {
  title: 'Branding/Nym Logo',
  component: NymLogo,
} as ComponentMeta<typeof NymLogo>;

export const Logo = () => <NymLogo height={250} />;

export const Icon = () => <NymIcon height={250} />;

export const Wordmark = () => <NymWordmark height={250} />;
