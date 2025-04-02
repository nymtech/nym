import { TBondedNode } from 'src/context/bonding';
import { isMixnode, isNymNode } from 'src/types';

// Update the NavItems type to include 'Cost Parameters'
export type NavItems = 'General' | 'Cost Parameters' | 'Unbond';

export const makeNavItems = (node: TBondedNode): NavItems[] => {
  // Basic tabs that are always available
  const tabs: NavItems[] = ['General'];

  // Add Cost Parameters tab for all node types
  tabs.push('Cost Parameters');

  // Add unbond tab for specific node types
  if (isMixnode(node) || isNymNode(node)) {
    tabs.push('Unbond');
  }

  return tabs;
};
