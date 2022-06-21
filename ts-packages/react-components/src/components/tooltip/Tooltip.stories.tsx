import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Tooltip } from './Tooltip';

export default {
  title: 'Basics/Tooltip',
  component: Tooltip,
} as ComponentMeta<typeof Tooltip>;

export const Default = () => <Tooltip title="tooltip" id="field-name" placement="top-start" arrow />;

export const NEStyle = () => {
  return (
    <Tooltip
      title="Figma NE Styles"
      id="field-name"
      placement="top-start"
      textColor="#A0AED1"
      bgColor="#A0AED1"
      maxWidth={230}
      arrow
    />
  );
};
