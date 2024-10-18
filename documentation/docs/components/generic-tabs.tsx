import { Tabs } from 'nextra/components';

export const MyTab = ({ name, children }) => (
  <Tabs.Tab>{name} {children}</Tabs.Tab>
);
