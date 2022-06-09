import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { DateTime } from 'luxon';
import { ConnectionStatus } from '../components/ConnectionStatus';
import { ConnectionStatusKind } from '../types';

export default {
  title: 'Components/ConnectionStatus',
  component: ConnectionStatus,
} as ComponentMeta<typeof ConnectionStatus>;

export const Disconnected: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={ConnectionStatusKind.disconnected} />
);

export const Connecting: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={ConnectionStatusKind.connecting} />
);

export const Connected: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={ConnectionStatusKind.connected} connectedSince={DateTime.now()} />
);

export const Disconnecting: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={ConnectionStatusKind.disconnecting} />
);
