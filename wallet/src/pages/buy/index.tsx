import React from 'react';
import { BuyContextProvider } from '@src/context';
import { Tutorial } from '@src/components/Buy/Tutorial';

export const BuyPage = () => (
  <BuyContextProvider>
    <Tutorial />
  </BuyContextProvider>
);
