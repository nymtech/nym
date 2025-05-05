export type MenuItem = {
  id: number;
  title: string;
  url: string;
};

const MENU_DATA: MenuItem[] = [
  {
    id: 1,
    title: "Explorer",
    url: "/table",
  },
  {
    id: 2,
    title: "Stake",
    url: "/stake",
  },
  {
    id: 3,
    title: "Onboarding",
    url: "/onboarding",
  },
];

export default MENU_DATA;
