import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { OverSaturatedBlockerModal } from './DelegateBlocker';

export default {
  title: 'Delegation/Components/Delegation Over Saturated Warning Modal',
  component: OverSaturatedBlockerModal,
} as ComponentMeta<typeof OverSaturatedBlockerModal>;

export const Default = () => <OverSaturatedBlockerModal open header="Node saturation: 114%" />;
