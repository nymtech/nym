'use client'

import React from 'react'
import { useIsMobile } from '@/app/hooks'
import { MobileNav } from './MobileNav'
import { Nav } from './Nav'

const Navbar = ({ children }: { children: React.ReactNode }) => {
  const isMobile = useIsMobile()

  if (isMobile) {
    return <MobileNav>{children}</MobileNav>
  }

  return <Nav>{children}</Nav>
}

export { Navbar }
