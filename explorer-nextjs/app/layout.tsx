import type { Metadata } from 'next'
import { Providers } from '@/app/providers'
import { Nav } from '@/app/components/Nav'

import '@interchain-ui/react/styles'

export const metadata: Metadata = {
  title: 'Nym Network Explorer',
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <Nav>{children}</Nav>
        </Providers>
      </body>
    </html>
  )
}
