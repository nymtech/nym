import * as React from 'react';
import { BIG_DIPPER } from '../api/constants';
import { OverviewSVG } from '../icons/OverviewSVG';
import { NodemapSVG } from '../icons/NodemapSVG';
import { NetworkComponentsSVG } from '../icons/NetworksSVG';

export type NavOptionType = {
  id: number;
  isActive?: boolean;
  url: string;
  title: string;
  Icon?: React.ReactNode;
  nested?: NavOptionType[];
  isExpandedChild?: boolean;
};

export const originalNavOptions: NavOptionType[] = [
  {
    id: 0,
    isActive: false,
    url: '/',
    title: 'Overview',
    Icon: <OverviewSVG />,
  },
  {
    id: 1,
    isActive: false,
    url: '/network-components',
    title: 'Network Components',
    Icon: <NetworkComponentsSVG />,
    nested: [
      {
        id: 3,
        url: '/network-components/mixnodes',
        title: 'Mixnodes',
      },
      {
        id: 4,
        url: '/network-components/gateways',
        title: 'Gateways',
      },
      {
        id: 5,
        url: `${BIG_DIPPER}/validators`,
        title: 'Validators',
      },
      {
        id: 6,
        url: 'network-components/service-providers',
        title: 'Service Providers',
      },
    ],
  },
  {
    id: 2,
    isActive: false,
    url: '/nodemap',
    title: 'Nodemap',
    Icon: <NodemapSVG />,
  },
];
