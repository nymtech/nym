export const makeNavItems = (isMixnode: boolean) => {
  const navItems: NavItems[] = ['General', 'Unbond'];

  if (isMixnode) navItems.splice(1, 0, 'Test my node', 'Playground');

  return navItems;
};

export type NavItems = 'General' | 'Unbond' | 'Test my node' | 'Playground';
