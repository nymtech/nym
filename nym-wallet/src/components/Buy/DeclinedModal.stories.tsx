import React from 'react';

import { DeclinedModal } from './DeclinedModal';

export default {
  title: 'Buy/DeclinedModal',
  component: DeclinedModal,
};

export const Terms = () => (
  <DeclinedModal onOk={async () => console.log('user has accepted')} onClose={async () => console.log('closed')} />
);
