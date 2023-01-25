import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { ConnectionButton } from '../components/ConnectionButton';
import { ConnectionStatusKind } from '../types';

export default {
  title: 'Components/ConnectionButton',
  component: ConnectionButton,
} as ComponentMeta<typeof ConnectionButton>;

export const Disconnected: ComponentStory<typeof ConnectionButton> = () => <ConnectionButton status={'disconnected'} />;

export const Connecting: ComponentStory<typeof ConnectionButton> = () => (
  <ConnectionButton status={'connecting'} busy />
);

export const Connected: ComponentStory<typeof ConnectionButton> = () => <ConnectionButton status={'connected'} />;

export const Disconnecting: ComponentStory<typeof ConnectionButton> = () => (
  <ConnectionButton status={'disconnecting'} busy />
);
