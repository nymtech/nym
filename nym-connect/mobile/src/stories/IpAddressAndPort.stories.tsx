import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { IpAddressAndPort } from '../components/IpAddressAndPort';

export default {
  title: 'Components/IpAddressAndPort',
  component: IpAddressAndPort,
} as ComponentMeta<typeof IpAddressAndPort>;

const Template: ComponentStory<typeof IpAddressAndPort> = (args) => <IpAddressAndPort {...args} />;

// 👇 Each story then reuses that template
export const Default = Template.bind({});
Default.args = { label: 'Socks5 address', ipAddress: '127.0.0.1', port: 1080 };
