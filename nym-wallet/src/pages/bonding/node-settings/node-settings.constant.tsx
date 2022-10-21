export const createNavItems = (isMixnode: boolean) => {
  const navItems = ['Unbond'];
  if (isMixnode) return ['General', ...navItems];
  return navItems;
};
