import React from 'react'
import { Navbar } from './components/Nav/Navbar'
import { Providers } from './providers'

const App = ({ children }: { children: React.ReactNode }) => (
  <Providers>
    <Navbar>{children}</Navbar>
  </Providers>
)

export { App }
