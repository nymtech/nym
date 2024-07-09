import React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { NetworkSelector, type Network } from '../../../lib/components/networks/NetworkSelector';

export default {
  title: 'Networks/Network Selector',
  component: NetworkSelector,
  argTypes: {
    network: {
      options: ['MAINNET', 'SANDBOX', 'QA'],
      control: { type: 'radio' },
    },
    onSwitchNetwork: { type: 'function' },
  },
} as ComponentMeta<typeof NetworkSelector>;

const Template: ComponentStory<typeof NetworkSelector> = ({ network: networkArg, onSwitchNetwork }) => {
  const [network, setNetwork] = React.useState<Network | undefined>(networkArg);
  const handleClick = (newNetwork?: Network) => {
    setNetwork(newNetwork);
    if (onSwitchNetwork && newNetwork) {
      onSwitchNetwork(newNetwork);
    }
  };
  return <NetworkSelector network={network || networkArg} onSwitchNetwork={handleClick} />;
};

export const Default = Template.bind({});

export const WithValue = Template.bind({});
WithValue.args = { network: 'MAINNET' };
