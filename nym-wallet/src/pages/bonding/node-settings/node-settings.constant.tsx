export const makeNavItems = (isMixnode: boolean) => {
  const navItems = ['General', 'Unbond'];

  if (isMixnode) navItems.splice(1, 0, 'Test', 'Playground');

  return navItems;
};
