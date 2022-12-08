import React from 'react';

import { Tutorial } from './Tutorial';
import { MockBuyContextProvider } from '../../context/mocks/buy';

export default {
  title: 'Buy/Tutorial',
  component: Tutorial,
};

export const TutorialPage = () => (
  <MockBuyContextProvider>
    <Tutorial />
  </MockBuyContextProvider>
);
