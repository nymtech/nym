export const createNavItems = (isMixnode: boolean) => {
  const navItems = ['Unbond'];
  if (isMixnode) return ['General', 'Playground', ...navItems];
  return navItems;
};
