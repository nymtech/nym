import type { Metadata } from 'next'
import '@interchain-ui/react/styles'
import { App } from './App'

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
        <App>{children}</App>
      </body>
    </html>
  )
}
