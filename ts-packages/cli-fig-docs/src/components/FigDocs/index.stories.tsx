import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { FigDocs } from './index';
import NymClientSpec from '../../../../../tools/nym-cli/user-docs/fig-spec';

export default {
  title: 'Docs/Fig',
  component: FigDocs,
} as ComponentMeta<typeof FigDocs>;

export const Default = () => <FigDocs />;

export const NymCli = () => <FigDocs figSpec={NymClientSpec} />;
