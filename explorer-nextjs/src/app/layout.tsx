import { Header } from "@/components/header";
import { Wrapper } from "@/components/wrapper";
import Providers from "@/providers";
import type { Metadata } from "next";

import "./globals.css";
import "@interchain-ui/react/styles";

export const metadata: Metadata = {
  title: "Nym Explorer V2",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <Header />
          <Wrapper>{children}</Wrapper>
        </Providers>
      </body>
    </html>
  );
}
