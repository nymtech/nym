import React from 'react';
import { DocsThemeConfig, useConfig } from 'nextra-theme-docs';
import { useRouter } from 'next/router';

const config: DocsThemeConfig = {
  logo: <span>Nym Typescript SDK</span>,
  project: {
    link: 'https://github.com/nymtech/nym',
  },
  chat: {
    link: 'https://discord.gg/nym',
  },
  docsRepositoryBase: 'https://github.com/nymtech/nym/tree/develop/sdk/typescript/docs',
  footer: {
    text: 'Nym Typescript SDK',
  },
  useNextSeoProps() {
    return {
      titleTemplate: '%s | Nym Typescript SDK',
    };
  },
  // head: () => {
  //   const { asPath, defaultLocale, locale } = useRouter();
  //   const { frontMatter } = useConfig();
  //   const url = `https://nymtech.net/docs/sdk/typescript/${defaultLocale === locale ? asPath : `/${locale}${asPath}`}`;
  //
  //   return (
  //     <>
  //       <meta property="og:url" content={url} />
  //       <meta property="og:title" content={frontMatter.title || 'Nym Typescript SDK'} />
  //       <meta
  //         property="og:description"
  //         content={
  //           frontMatter.description ||
  //           'The Nym Typescript SDK allows you to build Javascript and Typescript apps that send traffic over the Nym mixnet.'
  //         }
  //       />
  //     </>
  //   );
  // },
};

export default config;
