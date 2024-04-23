import * as React from 'react';
import { DelegateIcon } from '@src/icons/DelevateSVG';
import { BIG_DIPPER } from '../api/constants';
import { OverviewSVG } from '../icons/OverviewSVG';
import { NodemapSVG } from '../icons/NodemapSVG';
import { NetworkComponentsSVG } from '../icons/NetworksSVG';

export type NavOptionType = {
  isActive?: boolean;
  url: string;
  title: string;
  Icon?: React.ReactNode;
  nested?: NavOptionType[];
  isExpandedChild?: boolean;
};

export const originalNavOptions: NavOptionType[] = [
  {
    isActive: false,
    url: '/',
    title: 'Overview',
    Icon: <OverviewSVG />,
  },
  {
    isActive: false,
    url: '/network-components',
    title: 'Network Components',
    Icon: <NetworkComponentsSVG />,
    nested: [
      {
        url: '/network-components/mixnodes',
        title: 'Mixnodes',
      },
      {
        url: '/network-components/gateways',
        title: 'Gateways',
      },
      {
        url: `${BIG_DIPPER}/validators`,
        title: 'Validators',
      },
    ],
  },
  {
    isActive: false,
    url: '/nodemap',
    title: 'Nodemap',
    Icon: <NodemapSVG />,
  },
  {
    isActive: false,
    url: '/delegations',
    title: 'Delegations',
    Icon: <DelegateIcon />,
  },
];
