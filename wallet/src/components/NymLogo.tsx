import { NymLogo as NymLogoReact } from '@nymproject/react';

const imgSize = {
  small: 40,
  medium: 80,
  large: 120,
};

export const NymLogo = ({ size = 'medium' }: { size?: 'small' | 'medium' | 'large' }) => (
  <NymLogoReact width={imgSize[size]} />
);
