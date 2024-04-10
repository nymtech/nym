'use client'

import * as React from 'react'
import { DelegateIcon } from '@/app/icons/DelevateSVG'
import { BIG_DIPPER } from '@/app/api/constants'
import { OverviewSVG } from '@/app/icons/OverviewSVG'
import { NodemapSVG } from '@/app/icons/NodemapSVG'
import { NetworkComponentsSVG } from '@/app/icons/NetworksSVG'

export type NavOptionType = {
  isActive?: boolean
  url: string
  title: string
  Icon?: React.ReactNode
  nested?: NavOptionType[]
  isExpandedChild?: boolean
}

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
      {
        url: 'network-components/service-providers',
        title: 'Service Providers',
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
]
