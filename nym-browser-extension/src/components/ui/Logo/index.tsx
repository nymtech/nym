import React from 'react';
import { NymLogoBW } from '@nymproject/react/logo/NymLogoBW';

export const Logo = ({ small }: { small?: boolean }) => (
  <NymLogoBW width={small ? '37.5px' : '75px'} height={small ? '37.5px' : '75px'} />
);
