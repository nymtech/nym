import React from 'react';
import type { AppProps } from 'next/app';
import './styles.css';

const MyApp: React.FC<AppProps> = ({ Component, pageProps }) => {
  const AnyComponent = Component as any;
  return <AnyComponent {...pageProps} />;
};

export default MyApp;
