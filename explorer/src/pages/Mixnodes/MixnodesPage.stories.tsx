import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { MemoryRouter } from 'react-router-dom';
import { PageMixnodes } from './index';

export default {
  title: 'Mix Nodes Page',
  component: PageMixnodes,
  decorators: [
    (Story) => (
      <MemoryRouter>
        <Story />
      </MemoryRouter>
    ),
  ], //Wrapping the story inside the router
} as ComponentMeta<typeof PageMixnodes>;

const Template: ComponentStory<typeof PageMixnodes> = (args) => <PageMixnodes {...args} />;

export const empty = Template.bind({});
empty.args = {};
