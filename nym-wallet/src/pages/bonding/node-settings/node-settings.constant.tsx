import { TBondedNode } from 'src/context';
import { isNymNode } from 'src/types';

export const makeNavItems = (bondedNode: TBondedNode) => {
  const navItems: NavItems[] = ['Unbond'];

  if (isNymNode(bondedNode)) {
    // add these items to the beginning of the array "General", "Test my node", "Playground"
    navItems.unshift('General', 'Test my node', 'Playground');
  }

  return navItems;
};

export type NavItems = 'General' | 'Unbond' | 'Test my node' | 'Playground';
