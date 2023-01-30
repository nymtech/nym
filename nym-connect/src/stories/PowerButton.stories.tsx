import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { PowerButton } from 'src/components/PowerButton';

export default {
  title: 'Components/PowerButton',
  component: PowerButton,
} as ComponentMeta<typeof PowerButton>;

export const Disconnected: ComponentStory<typeof PowerButton> = () => <PowerButton status="disconnected" />;

export const Connecting: ComponentStory<typeof PowerButton> = () => <PowerButton status="connecting" />;

export const Connected: ComponentStory<typeof PowerButton> = () => <PowerButton status="connected" />;

export const Disconnecting: ComponentStory<typeof PowerButton> = () => <PowerButton status="disconnecting" />;
