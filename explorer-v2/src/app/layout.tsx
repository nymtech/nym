import { Header } from "@/components/header";
import { Banner } from "@/components/banner/Banner";
import { Wrapper } from "@/components/wrapper";
import Providers from "@/providers";
import type { Metadata } from "next";

import "./globals.css";
import "@interchain-ui/react/styles";
import { Footer } from "@/components/footer";

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
          <Banner />
          <Wrapper>{children}</Wrapper>
          <Footer />
        </Providers>
      </body>
    </html>
  );
}
