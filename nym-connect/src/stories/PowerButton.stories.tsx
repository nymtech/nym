import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { PowerButton } from 'src/components/PowerButton/PowerButton';
import { ConnectionStatusKind } from 'src/types';

export default {
  title: 'Components/PowerButton',
  component: PowerButton,
} as ComponentMeta<typeof PowerButton>;

export const Disconnected: ComponentStory<typeof PowerButton> = () => (
  <PowerButton status={ConnectionStatusKind.disconnected} />
);

export const Connecting: ComponentStory<typeof PowerButton> = () => (
  <PowerButton status={ConnectionStatusKind.connecting} disabled />
);

export const Connected: ComponentStory<typeof PowerButton> = () => (
  <PowerButton status={ConnectionStatusKind.connected} />
);

export const Disconnecting: ComponentStory<typeof PowerButton> = () => (
  <PowerButton status={ConnectionStatusKind.disconnecting} disabled />
);

export const Disabled: ComponentStory<typeof PowerButton> = () => (
  <PowerButton status={ConnectionStatusKind.disconnected} disabled />
);
