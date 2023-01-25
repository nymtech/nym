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
  <ConnectionStatus status={'disconnected'} gatewayPerformance="Good" />
);

export const Connecting: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={'connecting'} gatewayPerformance="Good" />
);

export const Connected: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={'connected'} connectedSince={DateTime.now()} gatewayPerformance="Good" />
);

export const Disconnecting: ComponentStory<typeof ConnectionStatus> = () => (
  <ConnectionStatus status={'disconnecting'} gatewayPerformance="Good" />
);
