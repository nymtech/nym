export const navItems = ['General', 'Unbond'] as const;

export type NodeSettingsNav = typeof navItems[number]; // type NodeSettingsNav = 'General' | 'Unbond';
