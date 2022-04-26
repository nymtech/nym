import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { EconomicsProgress } from './EconomicsProgress';

export default {
  title: 'Mix Node Detail/Economics/Progress',
  component: EconomicsProgress,
} as ComponentMeta<typeof EconomicsProgress>;

export const Empty = () => <EconomicsProgress />;
export const OverThreshold = () => <EconomicsProgress threshold={100} value={120} />;
export const UnderThreshold = () => <EconomicsProgress threshold={100} value={80} />;
export const OnThreshold = () => <EconomicsProgress threshold={100} value={100} />;
