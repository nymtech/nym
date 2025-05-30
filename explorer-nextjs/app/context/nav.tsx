'use client'

import * as React from 'react'
import { DelegateIcon } from '@/app/icons/DelevateSVG'
import { BLOCK_EXPLORER_BASE_URL } from '@/app/api/constants'
import { OverviewSVG } from '@/app/icons/OverviewSVG'
import { NodemapSVG } from '@/app/icons/NodemapSVG'
import { NetworkComponentsSVG } from '@/app/icons/NetworksSVG'

export type NavOptionType = {
  url: string
  title: string
  Icon?: React.ReactNode
  nested?: NavOptionType[]
  isExpandedChild?: boolean
  isExternal?: boolean
}

export const originalNavOptions: NavOptionType[] = [
  {
    url: '/',
    title: 'Overview',
    Icon: <OverviewSVG />,
  },
  {
    url: '/network-components',
    title: 'Network Components',
    Icon: <NetworkComponentsSVG />,
    nested: [
      {
        url: '/network-components/nodes',
        title: 'Nodes',
      },
      {
        url: '/network-components/mixnodes',
        title: 'Mixnodes (legacy)',
      },
      {
        url: '/network-components/gateways',
        title: 'Gateways (legacy)',
      },
      {
        url: `${BLOCK_EXPLORER_BASE_URL}/validators`,
        title: 'Validators',
        isExternal: true,
      },
    ],
  },
  {
    url: '/nodemap',
    title: 'Nodemap',
    Icon: <NodemapSVG />,
  },
  {
    url: '/delegations',
    title: 'Delegations',
    Icon: <DelegateIcon sx={{ color: 'white' }} />,
  },
]
