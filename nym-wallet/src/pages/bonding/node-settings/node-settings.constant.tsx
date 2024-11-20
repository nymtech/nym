import { TBondedNode } from 'src/context';
import { isNymNode } from 'src/types';

export const makeNavItems = (bondedNode: TBondedNode) => {
  const navItems: NavItems[] = ['Unbond'];

  if (isNymNode(bondedNode)) {
    // Add these items to the beginning of the array "General", "Test my node", "Playground"
    // Temporarily removed , 'Test my node due to wasm issues which we need to fix
    // 'Playground' due to freezing issues
    navItems.unshift('General');
  }

  return navItems;
};

// And these back in once fixed.
// 'Playground' | 'Test my node' include in array at a later point
export type NavItems = 'General' | 'Unbond';
