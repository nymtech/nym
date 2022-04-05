import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { ValidatorSettings } from 'src/pages';

export default {
  title: 'ValidatorSettings',
  component: ValidatorSettings,
} as ComponentMeta<typeof ValidatorSettings>;

export const AllControls = () => <ValidatorSettings />;