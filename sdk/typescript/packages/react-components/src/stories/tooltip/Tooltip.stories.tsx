import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Tooltip } from '@lib/components/tooltip';

export default {
  title: 'Basics/Tooltip',
  component: Tooltip,
} as ComponentMeta<typeof Tooltip>;

export const Default = () => <Tooltip title="tooltip" id="field-name" placement="top-start" arrow />;

export const NEStyle = () => (
  <Tooltip
    title="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 1 million NYM, computed as S/K where S is  total amount of tokens available to stakeholders and K is the number of nodes in the reward set."
    id="field-name"
    placement="top-start"
    textColor="#111826"
    bgColor="#A0AED1"
    maxWidth={230}
    arrow
  />
);
