enum Options {
  General,
  Unbond,
}
type NavItem = keyof typeof Options;

export const nodeSettingsNav: NavItem[] = ['General', 'Unbond'];
