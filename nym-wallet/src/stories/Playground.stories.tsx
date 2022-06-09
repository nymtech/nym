import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Playground } from '@nymproject/react/playground/Playground';

export default {
  title: 'Playground',
  component: Playground,
} as ComponentMeta<typeof Playground>;

export const AllControls = () => <Playground />;
