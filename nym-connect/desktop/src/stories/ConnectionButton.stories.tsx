import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { ConnectionStatusKind } from 'src/types';
import { ConnectionButton } from '../components/ConnectionButton';

export default {
  title: 'Components/ConnectionButton',
  component: ConnectionButton,
} as ComponentMeta<typeof ConnectionButton>;

export const Disconnected: ComponentStory<typeof ConnectionButton> = () => (
  <ConnectionButton status={ConnectionStatusKind.disconnected} />
);

export const Connecting: ComponentStory<typeof ConnectionButton> = () => (
  <ConnectionButton status={ConnectionStatusKind.connecting} busy />
);

export const Connected: ComponentStory<typeof ConnectionButton> = () => (
  <ConnectionButton status={ConnectionStatusKind.connected} />
);

export const Disconnecting: ComponentStory<typeof ConnectionButton> = () => (
  <ConnectionButton status={ConnectionStatusKind.disconnecting} busy />
);
